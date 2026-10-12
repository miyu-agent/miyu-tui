//! 输入框上边框右边写的那一截（蓝图 `tui.md`「长期目标」第 5 条、「切进子会话」第 2 条）：看着子会话的写子代理是谁，别的
//! 有长期目标的写「目标 · 进行中 · 第 2/5 轮」。

use crate::config::Config;
use crate::core::{Goal, GoalState};

/// 上边框右边写的（前后各空一格）：看着子代理的会话（`child` 是它的名字）写它是谁，目标不写；不然长期目标在跑、在等人开口、
/// 停着、卡住的写「目标 · 在哪一步 · 第几轮」，还没开过轮的不写第几轮；完成了、没有目标的不写。
pub(super) fn tag(child: Option<String>, goal: Option<&Goal>, config: &Config) -> Option<String> {
    if let Some(title) = child {
        return Some(config.text.jobs.child_tag.replace("{title}", &title));
    }
    let goal = goal?;
    let texts = &config.text.goal;
    let state = match goal.state {
        GoalState::Running => &texts.running,
        GoalState::Waiting => &texts.waiting,
        GoalState::Paused => &texts.paused,
        GoalState::Blocked => &texts.blocked,
        GoalState::Completed | GoalState::Other => return None,
    };
    let mut words = vec![texts.label.clone(), state.clone()];
    if goal.round > 0 {
        let rounds = match goal.limit {
            Some(limit) => texts.rounds_limited.replace("{limit}", &limit.to_string()),
            None => texts.rounds.clone(),
        };
        words.push(rounds.replace("{round}", &goal.round.to_string()));
    }
    Some(format!(" {} ", words.join(" · ")))
}

#[cfg(test)]
mod tests {
    use super::tag;
    use crate::config::Config;
    use crate::core::{Goal, GoalState};

    fn goal(state: GoalState, round: u32, limit: Option<u32>) -> Goal {
        Goal {
            state,
            round,
            limit,
        }
    }

    #[test]
    fn a_goal_shows_where_it_is_and_which_round() {
        // 2026-10-12 项目主人定：「目标 · 进行中 · 第 2/5 轮」，没上限的「第 2 轮」。
        let config = Config::builtin().unwrap();
        let shown = |g: Goal| tag(None, Some(&g), &config);
        assert_eq!(
            shown(goal(GoalState::Running, 2, Some(5))).as_deref(),
            Some(" 目标 · 进行中 · 第 2/5 轮 ")
        );
        assert_eq!(
            shown(goal(GoalState::Waiting, 1, None)).as_deref(),
            Some(" 目标 · 等待回复 · 第 1 轮 ")
        );
        assert_eq!(
            shown(goal(GoalState::Blocked, 3, None)).as_deref(),
            Some(" 目标 · 受阻 · 第 3 轮 ")
        );
        assert_eq!(
            shown(goal(GoalState::Paused, 0, Some(5))).as_deref(),
            Some(" 目标 · 已暂停 "),
            "还没开过轮的不写第 0 轮"
        );
    }

    #[test]
    fn a_finished_or_missing_goal_shows_nothing() {
        let config = Config::builtin().unwrap();
        let done = goal(GoalState::Completed, 3, None);
        assert_eq!(tag(None, Some(&done), &config), None);
        assert_eq!(tag(None, None, &config), None);
    }

    #[test]
    fn the_subagent_comes_before_the_goal() {
        // 「切进子会话」第 2 条：看着子会话时那里写子代理是谁。
        let config = Config::builtin().unwrap();
        let running = goal(GoalState::Running, 1, None);
        assert_eq!(
            tag(Some("查资料".into()), Some(&running), &config).as_deref(),
            Some(" 子代理 · 查资料 ")
        );
    }
}
