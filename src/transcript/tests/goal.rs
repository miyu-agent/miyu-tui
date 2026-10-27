//! 长期目标开的一轮（蓝图 `tui.md`「长期目标」第 4 条）：开它的那句不照原文画，画成暗色一行，归到它开的那一轮。

use super::apply;
use crate::core::{EndReason, Push};
use crate::transcript::{Kind, Transcript};

/// 长期目标开第 `round` 轮：那句是 30 号，这一轮是 31 号。
fn opened(round: Option<u32>) -> Transcript {
    let mut t = Transcript::default();
    apply(
        &mut t,
        vec![
            Push::GoalRound { seq: 30, round },
            Push::TurnStarted(31, Some(30)),
            Push::TurnEnded(EndReason::Completed),
        ],
    );
    t
}

#[test]
fn a_round_the_goal_opened_is_one_line_in_that_round() {
    let t = opened(Some(3));
    let line = t.entries.iter().find(|e| e.kind == Kind::Goal);
    let line = line.expect("画了一行");
    assert_eq!(
        (line.mark.as_deref(), line.text.as_str(), line.turn),
        (Some("● "), "长期目标 · 第 3 轮", Some(31))
    );
    assert!(
        t.entries.iter().all(|e| e.kind != Kind::User),
        "不当你说的话画"
    );
    assert!(t.last_said().is_none(), "不是你说的话开的那一轮不能重做");
}

#[test]
fn a_round_without_a_number_says_only_goal() {
    let t = opened(None);
    let line = t.entries.iter().find(|e| e.kind == Kind::Goal).unwrap();
    assert_eq!(line.text, "长期目标");
}

#[test]
fn undoing_the_round_takes_its_line_along() {
    // 技术定（2026-10-12）：那一行是那一轮的开头，撤掉那一轮一起藏起；别处来的话不跟着撤（「别处来的话」第 4 条）。
    let mut t = opened(Some(2));
    apply(&mut t, vec![Push::Reverted(vec![31])]);
    let line = t.entries.iter().find(|e| e.kind == Kind::Goal).unwrap();
    assert!(line.hidden);
}

#[test]
fn the_goal_is_kept_as_it_was_last_pushed() {
    // 核心 X-8 补：订阅回应里的、推来的 `goal.changed` 照最后一条换；`null` 是清掉了。
    use crate::core::{Goal, GoalState};
    let mut t = Transcript::default();
    let running = Goal {
        state: GoalState::Running,
        round: 1,
        limit: None,
        ..Goal::default()
    };
    apply(&mut t, vec![Push::Goal(Some(running.clone()))]);
    assert_eq!(t.goal, Some(running));
    apply(&mut t, vec![Push::Goal(None)]);
    assert_eq!(t.goal, None);
}
