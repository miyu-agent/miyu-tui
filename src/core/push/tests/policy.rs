//! 会话中途换人格、预设（蓝图 `tui.md`「新会话：人格、工作区」第 2a 条，核心 P-13）：`session.policy_changed` 多 `persona`、
//! `preset` 两格，`persona` 写空的是换成无人格。

use serde_json::json;

use super::read_mine;
use crate::core::push::Push;

#[test]
fn a_switched_persona_and_preset_are_read_and_an_empty_persona_means_none() {
    let event = json!({"seq": 9, "kind": "session.policy_changed", "by": {"kind": "person"},
        "body": {"persona": "xiaoxue", "preset": "coding"}});
    assert_eq!(
        read_mine(&event),
        [
            Push::PersonaSet("xiaoxue".into()),
            Push::PresetSet("coding".into())
        ]
    );
    let none = json!({"seq": 10, "kind": "session.policy_changed", "by": {"kind": "person"},
        "body": {"persona": ""}});
    assert_eq!(read_mine(&none), [Push::PersonaSet(String::new())]);
}

#[test]
fn compactable_changed_says_whether_there_is_something_to_compact() {
    // 核心 W-12 补：瞬时的 `compactable.changed`，换人格、预设前写不写「切换前会先压缩」照它（2026-10-20 项目主人）。
    let event = |on: bool| {
        serde_json::json!({"kind": "compactable.changed", "by": {"kind": "kernel"},
            "body": {"compactable": on}})
    };
    assert_eq!(super::read_mine(&event(true)), [Push::Compactable(true)]);
    assert_eq!(super::read_mine(&event(false)), [Push::Compactable(false)]);
}

/// 会话钉的压缩触发线（核心 CT-1）：`compaction_line` 是 `{"percent"}`，`null` 是不再钉着。
#[test]
fn a_pinned_compaction_line_is_read_and_null_unpins() {
    let event = |line: serde_json::Value| {
        json!({"seq": 11, "kind": "session.policy_changed", "by": {"kind": "person"},
            "body": {"compaction_line": line}})
    };
    assert_eq!(
        read_mine(&event(json!({"percent": 70}))),
        [Push::CompactionLineSet(Some(70))]
    );
    assert_eq!(
        read_mine(&event(json!(null))),
        [Push::CompactionLineSet(None)]
    );
}

/// 核心 TT-3（2026-11-13）：人要的重起标题没起成，`session.retitle_failed` 带给人看的那一句和那一次的错。
#[test]
fn a_failed_retitle_carries_the_line_and_the_error() {
    let event = json!({"seq": 12, "kind": "session.retitle_failed", "by": {"kind": "person"},
        "body": {"error": {"class": "timeout", "message": " timed out "},
            "human": {"key": "core/sessions/retitle-failed"}}});
    let got = read_mine(&event);
    let [Push::RetitleFailed { said, error }] = got.as_slice() else {
        panic!("{got:?}");
    };
    assert_eq!(
        said.as_ref().map(|s| s.key.as_str()),
        Some("core/sessions/retitle-failed")
    );
    assert_eq!(error, "timed out");
}

/// 认不出的级别（以前的 `workspace` 也是）照核心按最严的算（AUD-4，`kernel/events-bodies.md`「权限」）：画成只读，不是不画。
#[test]
fn an_unknown_level_is_drawn_read_only() {
    let event = |level: &str| {
        json!({"seq": 12, "kind": "session.policy_changed", "by": {"kind": "person"},
            "body": {"permission": {"level": level, "read_only": false}}})
    };
    let level = |level: &str| match read_mine(&event(level)).as_slice() {
        [
            Push::Policy {
                level,
                read_only: false,
            },
        ] => Some(*level),
        _ => None,
    };
    use crate::core::Level;
    assert_eq!(level("full"), Some(Level::Full));
    assert_eq!(level("workspace"), Some(Level::ReadOnly));
    assert_eq!(level("sandbox"), Some(Level::ReadOnly));
}
