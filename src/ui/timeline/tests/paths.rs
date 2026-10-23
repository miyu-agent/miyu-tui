//! 读取、编辑、写入标题里的路径能点（蓝图「时间线」第 7a 条）：文件真在的才算，列落在路径那一截上，截掉的不算。

use std::time::Instant;

use serde_json::json;
use unicode_width::UnicodeWidthStr;

use super::super::step;
use super::step as make;
use crate::core::ToolStatus;
use crate::theme;
use crate::transcript::{Step, StepKind, ToolState};
use crate::ui::test_support::Fixture;

fn tool(name: &str, path: &str) -> Step {
    make(
        StepKind::Tool {
            name: name.into(),
            args: String::new(),
            parsed: json!({"path": path}),
            state: ToolState::Done(ToolStatus::Ok),
            output: String::new(),
            said: None,
        },
        Instant::now(),
        0,
        1,
    )
}

#[test]
fn the_path_in_a_read_edit_or_write_title_opens_when_the_file_is_there() {
    let dir = std::env::temp_dir().join(format!("miyu-tui-step-paths-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("a.rs");
    std::fs::write(&file, "x").unwrap();
    let file = file.canonicalize().unwrap().display().to_string();
    let mut f = Fixture::new();
    let face = |name: &str| json!({"name": name, "subject": "path"});
    f.human = crate::human::Human::from_reply(&json!({"language": "zh", "tools": {
        "read": face("读取"), "edit": face("编辑"), "write": face("写入"), "glob": face("找文件")}}));
    let ctx = f.ctx();
    for name in ["read", "edit", "write"] {
        let (spans, links) = step::title(&tool(name, &file), theme::dim(), 200, &ctx);
        let line: String = spans.iter().map(|s| s.content.as_ref()).collect();
        let [(from, to, url)] = links.as_slice() else {
            panic!("{name}：{links:?}");
        };
        assert_eq!(url, &file);
        let shown = crate::local::home_short(&file);
        let at = line.find(&shown).unwrap();
        assert_eq!(usize::from(*from), line[..at].width(), "{name}");
        assert_eq!(
            usize::from(*to),
            line[..at].width() + shown.width(),
            "{name}"
        );
    }
    let (_, links) = step::title(&tool("glob", &file), theme::dim(), 200, &ctx);
    assert!(links.is_empty(), "别的工具不算");
    let gone = format!("{file}.gone");
    let (_, links) = step::title(&tool("read", &gone), theme::dim(), 200, &ctx);
    assert!(links.is_empty(), "文件不在的不算");
    // 窄到路径整截被截掉：不算链接；截了一半：只算露着的。
    let (_, links) = step::title(&tool("read", &file), theme::dim(), 8, &ctx);
    assert!(links.is_empty(), "{links:?}");
    let (spans, links) = step::title(&tool("read", &file), theme::dim(), 16, &ctx);
    let line: String = spans.iter().map(|s| s.content.as_ref()).collect();
    if let [(_, to, _)] = links.as_slice() {
        assert!(usize::from(*to) < line.width(), "`…` 不算：{line}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
