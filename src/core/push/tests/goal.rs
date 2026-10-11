//! 长期目标开的一轮（蓝图 `tui.md`「长期目标」第 4 条，核心 X-7、X-8）：`message.user` 是模块 `goal` 发的，第几轮照它的 `cause`。

use serde_json::json;

use super::read_mine;
use crate::core::push::Push;

/// 长期目标发的一句，命令编号是 `cause`。
fn round(cause: &str) -> serde_json::Value {
    json!({"seq": 30, "kind": "message.user", "by": {"kind": "module", "id": "goal"}, "cause": cause,
        "body": {"blocks": [{"type": "text", "text": "<goal_round>接着做</goal_round>"}]}})
}

#[test]
fn a_round_the_goal_opened_is_read_with_its_number() {
    // 核心照 `goal-<第几轮>-<毫秒>` 造命令编号（`actor/goal.rs`）。
    assert_eq!(
        read_mine(&round("goal-3-1760252400000")),
        [Push::GoalRound {
            seq: 30,
            round: Some(3)
        }]
    );
}

#[test]
fn a_round_whose_cause_reads_otherwise_has_no_number() {
    for cause in ["goal-call_7", "goal-x-1", "goal-3", "web-3-1"] {
        assert_eq!(
            read_mine(&round(cause)),
            [Push::GoalRound {
                seq: 30,
                round: None
            }],
            "{cause}"
        );
    }
}

#[test]
fn another_module_is_still_from_elsewhere() {
    let event = json!({"seq": 31, "kind": "message.user", "by": {"kind": "module", "id": "memory"},
        "cause": "m-1", "body": {"blocks": [{"type": "text", "text": "想起来了"}]}});
    assert!(
        matches!(read_mine(&event)[..], [Push::Foreign(_)]),
        "{:?}",
        read_mine(&event)
    );
}
