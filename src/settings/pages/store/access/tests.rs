use serde_json::json;

use super::*;

#[test]
fn source_rows_read_kind_and_provides() {
    assert_eq!(
        of(&json!({"kind": "bundle", "provides": {"scripts": ["ocr"]}})),
        ["scripts"]
    );
    assert_eq!(
        of(
            &json!({"kind": "bundle", "provides": {"skills": ["a"], "mcp": ["notes"], "scripts": []}})
        ),
        ["mcp", "skills"],
        "空的种类不算"
    );
    assert_eq!(
        of(
            &json!({"kind": "process", "provides": {"command": ["im"], "skills": ["x"], "scripts": ["y"], "personas": ["p"]}})
        ),
        ["program", "skills", "scripts"],
        "照先后最多三个"
    );
    assert_eq!(of(&json!({"kind": "theme"})), ["theme"]);
    assert_eq!(
        of(&json!({"provides": {"personas": true}})),
        ["personas"],
        "真假也认"
    );
}

#[test]
fn installed_items_read_connection_mcp_and_command() {
    assert_eq!(
        of(&json!({"kind": "process", "connection": {"platform": "qq"}, "mcp": [{"name": "x"}]})),
        ["connection", "program", "mcp"]
    );
    assert_eq!(
        of(&json!({"kind": "bundle", "command": {"name": "memes"}})),
        ["program"],
        "加子命令的算带程序"
    );
    assert_eq!(of(&json!({"kind": "worker"})), ["program"]);
    assert_eq!(of(&json!({"kind": "ui"})), ["ui"]);
    assert_eq!(of(&json!({"kind": "mascot"})), ["mascot"]);
}

#[test]
fn built_in_packages_have_no_labels() {
    assert!(of(&json!({"kind": "builtin", "mcp": [], "provides": {}})).is_empty());
    assert!(of(&json!({})).is_empty());
}
