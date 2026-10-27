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

/// 框上面那一行写的：长期目标在跑、在等人开口、停着、卡住的写「◎ 目标 · 在哪一步 · 第几轮 · 用时」，还没开过轮的不写
/// 第几轮；用时照设下的时刻算到 `now`（2026-10-26 项目主人：右边加计时），画一帧算一次，空闲时不另起表重画。做完了、
/// 没有目标的不写。看着子会话时不写（调用的那一处管）。点它开浮窗（`goal_box.rs`）。
pub(crate) fn goal(goal: Option<&Goal>, config: &Config, now: jiff::Timestamp) -> Option<String> {
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
    if let Some(since) = goal.since {
        words.push(crate::meter::clock(elapsed(since, now)));
    }
    Some(format!(
        "{}{}",
        config.layout.goal_mark,
        words.join(&crate::sheet::sep())
    ))
}

/// 从 `since` 到 `now` 过了几秒；钟往回拨了的算 0。
pub(crate) fn elapsed(since: jiff::Timestamp, now: jiff::Timestamp) -> u64 {
    u64::try_from(now.as_second() - since.as_second()).unwrap_or_default()
}

/// 画在框上面那一行，右对齐、暗色：右端和框下面那一行的用量对齐（`area` 是框里放字那么宽的一行）。点得动（开浮窗），
/// 指针停在字上时加下划线（2026-10-27 项目主人报：原来悬停没下划线、指针不变手）。
pub(super) fn draw(frame: &mut Frame, area: Rect, text: &str, hover: bool) {
    let style = if hover {
        theme::dim().add_modifier(ratatui::style::Modifier::UNDERLINED)
    } else {
        theme::dim()
    };
    frame.render_widget(
        Paragraph::new(Line::styled(text.to_string(), style).right_aligned()),
        area,
    );
}

/// 字占的那一截：右对齐、一行（点它、悬停照它认）。
pub(crate) fn spot(area: Rect, text: &str) -> Rect {
    let width = u16::try_from(unicode_width::UnicodeWidthStr::width(text))
        .unwrap_or(u16::MAX)
        .min(area.width);
    Rect::new(area.right() - width, area.y, width, area.height.min(1))
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
            ..Goal::default()
        }
    }

    #[test]
    fn a_goal_shows_where_it_is_and_which_round() {
        // 2026-10-12 项目主人定：「目标 · 进行中 · 第 2/5 轮」，没上限的「第 2 轮」；2026-10-14 打头加 `◎`。
        let config = Config::builtin().unwrap();
        let now = jiff::Timestamp::UNIX_EPOCH;
        let shown = |g: Goal| goal(Some(&g), &config, now);
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
        // 右边接用时，照设下的时刻算（2026-10-26 项目主人）。
        let mut timed = made(GoalState::Running, 2, Some(5));
        timed.since = Some(jiff::Timestamp::UNIX_EPOCH);
        let later = jiff::Timestamp::from_second(65).unwrap();
        assert_eq!(
            goal(Some(&timed), &config, later).as_deref(),
            Some("◎ 目标 · 进行中 · 第 2/5 轮 · 1m 05s")
        );
    }

    #[test]
    fn a_finished_or_missing_goal_shows_nothing() {
        let config = Config::builtin().unwrap();
        let done = made(GoalState::Completed, 3, None);
        let now = jiff::Timestamp::UNIX_EPOCH;
        assert_eq!(goal(Some(&done), &config, now), None);
        assert_eq!(goal(None, &config, now), None);
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

    #[test]
    fn hovering_the_goal_underlines_it() {
        // 2026-10-27 项目主人报：原来悬停没下划线。字占的那一截右对齐；悬停时整截加下划线。
        use ratatui::layout::Rect;
        use ratatui::style::Modifier;
        let area = Rect::new(0, 0, 30, 1);
        let spot = super::spot(area, "目标 2/5");
        assert_eq!((spot.x, spot.width), (22, 8), "右对齐");
        for hover in [false, true] {
            let backend = ratatui::backend::TestBackend::new(30, 1);
            let mut terminal = ratatui::Terminal::new(backend).unwrap();
            terminal
                .draw(|f| super::draw(f, area, "目标 2/5", hover))
                .unwrap();
            let cell = &terminal.backend().buffer()[(22, 0)];
            assert_eq!(cell.modifier.contains(Modifier::UNDERLINED), hover);
        }
    }
}
