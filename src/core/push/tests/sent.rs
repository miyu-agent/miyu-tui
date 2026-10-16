//! 她发出去的图（`tool.result` 的 `sent`，核心 X-19）。

use serde_json::json;

use super::read_mine;
use crate::core::{Push, Sent};

#[test]
fn pictures_she_sent_come_after_the_result_and_odd_ones_are_left_out() {
    let result = json!({"kind": "tool.result", "by": {"kind": "tool"},
        "body": {"call_id": "c1", "status": "ok", "blocks": [{"type": "text", "text": "Sent."}],
            "sent": [
                {"blob": "sha256:aa", "media_type": "image/png", "width": 240, "height": 200,
                    "name": "shrug.png", "alt": "摊手", "sticker": true},
                {"blob": "sha256:bb", "media_type": "image/jpeg", "width": 800, "height": 600},
                {"media_type": "image/png", "width": 1, "height": 1},
                {"blob": "sha256:cc", "media_type": "audio/ogg"}]}});
    let got = read_mine(&result);
    assert!(matches!(&got[0], Push::ToolResult { .. }), "结果照旧先交");
    assert_eq!(
        got[1],
        Push::Media(vec![
            Sent {
                blob: "sha256:aa".into(),
                name: Some("shrug.png".into()),
                alt: Some("摊手".into()),
                sticker: true,
            },
            Sent {
                blob: "sha256:bb".into(),
                name: None,
                alt: None,
                sticker: false,
            },
        ]),
        "没有 blob 的、不是图的不要"
    );
    assert_eq!(got.len(), 2);
    let old = json!({"kind": "tool.result", "by": {"kind": "tool"},
        "body": {"call_id": "c2", "status": "ok", "blocks": []}});
    assert_eq!(read_mine(&old).len(), 1, "老日志没有这一格，不另交");
}
