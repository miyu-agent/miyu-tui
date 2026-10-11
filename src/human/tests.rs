//! 给人看的字照 `human.get` 的回应造，和照资源目录读的说得一样（核心 W-1）。

use serde_json::json;

use super::Human;
use miyu_client::protocol::Said;

fn said(key: &str, fields: &[(&str, &str)]) -> Said {
    Said {
        key: key.into(),
        fields: fields
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect(),
    }
}

#[test]
fn the_reply_gives_tool_faces_and_fills_phrases() {
    let human = Human::from_reply(&json!({
        "language": "zh",
        "said": {"software/basesystem/read/lines": "{count} 行", "core/bad": "{没收尾"},
        "tools": {"read": {"name": "读取", "subject": "file_path"}, "odd": 3}
    }));
    let face = human.tool("read").unwrap();
    assert_eq!(
        (face.name.as_str(), face.subject.as_deref()),
        ("读取", Some("file_path"))
    );
    assert!(human.tool("odd").is_none(), "读不懂的那一件不要");
    assert_eq!(
        human
            .say(&said("software/basesystem/read/lines", &[("count", "3")]))
            .as_deref(),
        Some("3 行")
    );
    assert_eq!(human.say(&said("core/bad", &[])), None, "坏模板那一句不要");
    assert_eq!(Human::default().tool("read"), None, "没要到的是空的");
}

#[test]
fn an_mcp_tool_without_a_face_is_named_by_its_service_and_tool() {
    let human = Human::default();
    assert_eq!(
        human.tool_name("mcp__github__create_issue"),
        "github · create_issue"
    );
    assert_eq!(
        human.tool_name("mcp__fs__read__file"),
        "fs · read__file",
        "工具名里带 __ 的照原样"
    );
    assert_eq!(human.tool_name("mcp____x"), "mcp____x", "服务名空着的不认");
    assert_eq!(human.tool_name("read"), "read");
    let named = Human::from_reply(&serde_json::json!({"tools": {"read": {"name": "读取"}}}));
    assert_eq!(named.tool_name("read"), "读取");
}
