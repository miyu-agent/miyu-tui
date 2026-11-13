//! 她给人看的文件（`tool.result` 的效果 `file.shown`，核心 D-5，蓝图「展示」第 1 条）。

use serde_json::json;

use super::read_mine;
use crate::core::{Push, Shown};

#[test]
fn a_shown_file_comes_after_the_result_and_odd_ones_are_left_out() {
    let result = json!({"kind": "tool.result", "by": {"kind": "tool"},
        "body": {"call_id": "c1", "status": "ok", "blocks": [{"type": "text", "text": "Shown to the user."}],
            "effects": [
                {"kind": "file.shown", "path": "/w/index.html", "name": "index.html",
                    "media_type": "text/html", "size": 4200, "modified": "2026-11-13T01:00:00Z", "title": "首页"},
                {"kind": "file.shown", "path": "/w/a.rs", "name": "a.rs",
                    "media_type": "text/plain", "size": 10, "language": "rust"},
                {"kind": "file.shown", "name": "no-path.png"},
                {"kind": "job.messaged", "job": "j1"}]}});
    let got = read_mine(&result);
    let result_at = got
        .iter()
        .position(|p| matches!(p, Push::ToolResult { .. }));
    let shown_at = got.iter().position(|p| matches!(p, Push::Shown(_)));
    assert!(result_at < shown_at, "结果先交，展示排在后面：{got:?}");
    let shown: Vec<&Shown> = got
        .iter()
        .filter_map(|p| match p {
            Push::Shown(file) => Some(file),
            _ => None,
        })
        .collect();
    assert_eq!(shown.len(), 2, "缺 path 的不要（核心 SW-1 起没有 blob）");
    assert_eq!(shown[0].modified.as_deref(), Some("2026-11-13T01:00:00Z"));
    assert_eq!(shown[0].title.as_deref(), Some("首页"));
    assert!(shown[0].opens_at_once() && !shown[0].image());
    assert_eq!(shown[1].language.as_deref(), Some("rust"));
    assert_eq!((shown[1].size, shown[1].name.as_str()), (10, "a.rs"));
}
