//! 输入框上边框右边、框上面那一行右边写的（蓝图 `tui.md`「长期目标」第 5 条、「切进子会话」第 2 条、「输入框」第 14 条）：
//! 上边框右边是「这是哪个会话」的位置，看着子会话的写子代理是谁，不然人起过名字（`/rename`）的写会话标题；长期目标的进度
//! 写在框上面那一行，右对齐，打头一个 `◎`（2026-10-14 项目主人定：原来写在上边框右边，占了标题的位置）。

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::config::Config;
use crate::core::{Goal, GoalState};
use crate::theme;

/// 上边框右边写的（前后各空一格）：看着子代理的会话（`child` 是它的名字）写它是谁；不然人起过名字的（`named`：`/rename`
/// 改的，不是内核自动起的）写标题，长了截掉加 `…`（2026-10-13 项目主人定：照 Claude Code，标题不一直写）。
pub(super) fn border(
    child: Option<String>,
    named: Option<&str>,
    config: &Config,
) -> Option<String> {
    if let Some(title) = child {
        return Some(config.text.jobs.child_tag.replace("{title}", &title));
    }
    named.map(|t| {
        format!(
            " {} ",
            crate::cells::clip(t, config.layout.session_title_width)
        )
    })
}

/// 框上面那一行写的：长期目标在跑、在等人开口、停着、卡住的写「◎ 目标 · 在哪一步 · 第几轮」，还没开过轮的不写第几轮；
/// 做完了、没有目标的不写。看着子会话时不写（调用的那一处管）。
pub(super) fn goal(goal: Option<&Goal>, config: &Config) -> Option<String> {
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
    Some(format!("{}{}", config.layout.goal_mark, words.join(" · ")))
}

/// 画在框上面那一行，右对齐、暗色：右端和框下面那一行的用量对齐（`area` 是框里放字那么宽的一行）。
pub(super) fn draw(frame: &mut Frame, area: Rect, text: &str) {
    frame.render_widget(
        Paragraph::new(Line::styled(text.to_string(), theme::dim()).right_aligned()),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::{border, goal};
    use crate::config::Config;
    use crate::core::{Goal, GoalState};

    fn made(state: GoalState, round: u32, limit: Option<u32>) -> Goal {
        Goal {
            state,
            round,
            limit,
        }
    }

    #[test]
    fn a_goal_shows_where_it_is_and_which_round() {
        // 2026-10-12 项目主人定：「目标 · 进行中 · 第 2/5 轮」，没上限的「第 2 轮」；2026-10-14 打头加 `◎`。
        let config = Config::builtin().unwrap();
        let shown = |g: Goal| goal(Some(&g), &config);
        assert_eq!(
            shown(made(GoalState::Running, 2, Some(5))).as_deref(),
            Some("◎ 目标 · 进行中 · 第 2/5 轮")
        );
        assert_eq!(
            shown(made(GoalState::Waiting, 1, None)).as_deref(),
            Some("◎ 目标 · 等待回复 · 第 1 轮")
        );
        assert_eq!(
            shown(made(GoalState::Blocked, 3, None)).as_deref(),
            Some("◎ 目标 · 受阻 · 第 3 轮")
        );
        assert_eq!(
            shown(made(GoalState::Paused, 0, Some(5))).as_deref(),
            Some("◎ 目标 · 已暂停"),
            "还没开过轮的不写第 0 轮"
        );
    }

    #[test]
    fn a_finished_or_missing_goal_shows_nothing() {
        let config = Config::builtin().unwrap();
        let done = made(GoalState::Completed, 3, None);
        assert_eq!(goal(Some(&done), &config), None);
        assert_eq!(goal(None, &config), None);
    }

    #[test]
    fn the_border_says_which_session_it_is_never_the_goal() {
        // 「切进子会话」第 2 条：看着子会话时写子代理是谁；不然写人起的标题，长的截掉加「…」（2026-10-13 项目主人定）。
        // 2026-10-14 项目主人定：上边框右边是标题的位置，目标挪到框上面那一行，有目标在跑时标题照写。
        let config = Config::builtin().unwrap();
        assert_eq!(
            border(Some("查资料".into()), Some("改过的名字"), &config).as_deref(),
            Some(" 子代理 · 查资料 ")
        );
        assert_eq!(
            border(None, Some("验收会话"), &config).as_deref(),
            Some(" 验收会话 ")
        );
        assert_eq!(border(None, None, &config), None, "内核起的、没标题的不写");
        let long = "很长".repeat(30);
        let shown = border(None, Some(&long), &config).unwrap();
        assert!(shown.ends_with("… "), "{shown}");
    }
}
