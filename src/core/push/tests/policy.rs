//! 会话中途换人格、预设（蓝图 `tui.md`「新会话：人格、工作区」第 2a 条，核心 P-13）：`session.policy_changed` 多 `persona`、
//! `preset` 两格，`persona` 写空的是换成无人格。

use serde_json::json;

use super::read_mine;
use crate::core::push::Push;

#[test]
fn a_switched_persona_and_preset_are_read_and_an_empty_persona_means_none() {
    let event = json!({"seq": 9, "kind": "session.policy_changed", "by": {"kind": "person", "account": "admin"},
        "body": {"persona": "xiaoxue", "preset": "coding"}});
    assert_eq!(
        read_mine(&event),
        [
            Push::PersonaSet("xiaoxue".into()),
            Push::PresetSet("coding".into())
        ]
    );
    let none = json!({"seq": 10, "kind": "session.policy_changed", "by": {"kind": "person", "account": "admin"},
        "body": {"persona": ""}});
    assert_eq!(read_mine(&none), [Push::PersonaSet(String::new())]);
}
