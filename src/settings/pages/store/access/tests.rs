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

/// 出厂的内置包打头写「内置」，后面接它带的（核心 PV-2 起 package.list 带 provides；2026-11-12 项目主人：两个头都写内置）。
#[test]
fn built_in_packages_lead_with_built_in_and_then_what_they_carry() {
    assert_eq!(
        of(&json!({"kind": "builtin", "mcp": [], "provides": {}})),
        ["builtin"]
    );
    assert_eq!(
        of(
            &json!({"kind": "builtin", "provides": {"skills": ["a"], "scripts": ["b"], "knowledge": ["c"]}})
        ),
        ["builtin", "skills", "scripts"],
        "一共最多三个"
    );
    assert!(of(&json!({})).is_empty());
}

/// 核心 PV-3：provides 多知识库。
#[test]
fn a_knowledge_base_is_a_label_too() {
    assert_eq!(
        of(&json!({"kind": "bundle", "provides": {"knowledge": ["recipes"]}})),
        ["knowledge"]
    );
}
