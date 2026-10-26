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

/// 推来的 `goal.changed`（核心 X-8 补，瞬时事件）。
fn changed(goal: serde_json::Value) -> serde_json::Value {
    json!({"at": "2026-10-12T08:00:00Z", "kind": "goal.changed", "by": {"kind": "module", "id": "goal"},
        "body": {"goal": goal}})
}

#[test]
fn a_changed_goal_is_read_as_its_state_round_and_limit() {
    use crate::core::push::{Goal, GoalState};
    let goal = json!({"text": "把测试修绿", "state": "running", "round": 2, "limit": 5,
        "since": "2026-10-12T07:00:00Z", "usage": {}});
    assert_eq!(
        read_mine(&changed(goal)),
        [Push::Goal(Some(Goal {
            state: GoalState::Running,
            round: 2,
            limit: Some(5),
            text: "把测试修绿".into(),
            since: "2026-10-12T07:00:00Z".parse().ok(),
            ..Goal::default()
        }))]
    );
    let waiting = json!({"text": "t", "state": "waiting", "round": 1, "since": "x", "usage": {}});
    assert_eq!(
        read_mine(&changed(waiting)),
        [Push::Goal(Some(Goal {
            state: GoalState::Waiting,
            round: 1,
            limit: None,
            text: "t".into(),
            ..Goal::default()
        }))]
    );
    assert_eq!(
        read_mine(&changed(json!(null))),
        [Push::Goal(None)],
        "清掉了"
    );
}

#[test]
fn a_goals_usage_and_reason_are_read_for_its_box() {
    // 2026-10-26 项目主人：点开目标那一行的浮窗写用量；卡住的写为什么。
    use crate::core::push::Goal;
    let goal = json!({"text": "t", "state": "blocked", "round": 3, "reason": "缺权限",
        "since": "2026-10-12T07:00:00Z",
        "usage": {"usage": {"uncached": 100, "cache_read": 900, "cache_write": 0, "output": 50},
            "amounts": [{"currency": "CNY", "amount": 0.2}]}});
    let pushes = read_mine(&changed(goal));
    let [
        Push::Goal(Some(Goal {
            reason,
            tokens,
            bill,
            ..
        })),
    ] = pushes.as_slice()
    else {
        panic!("读成一份目标");
    };
    assert_eq!(reason.as_deref(), Some("缺权限"));
    assert_eq!(*tokens, 1050);
    assert_eq!(bill.amounts.len(), 1);
}
