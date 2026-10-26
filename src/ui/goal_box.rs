//! 长期目标的浮窗（蓝图 `tui.md`「长期目标」第 5a 条，2026-10-26 项目主人定：点目标那一行开，同 `/status` 的浮窗）：
//! 一行一项，左边一列名字对齐，右边写值，放不下的折到下一行、照值那一列对齐：目标全文、状态（卡住的接为什么）、
//! 轮数（有上限的写「2/5」）、用时、用量（token 和花费）；状态后面接恢复次数（核心 GOAL-1，0 次不写）。`↑` `↓` `PgUp`
//! `PgDn` 滚，`Esc` 关。核心没给的不写。

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::panel::{self, Chrome};
use crate::app::Seen;
use crate::config::Config;
use crate::core::{Goal, GoalState};
use crate::input::wrap_words;
use crate::theme;

/// 框和露出来的行：从第 `scroll` 行起最多 `max` 行；没有目标的一行都没有。
pub fn lines(
    seen: &Seen,
    scroll: usize,
    width: u16,
    max: usize,
) -> (Chrome, Vec<Line<'static>>, Vec<Option<usize>>) {
    let texts = &seen.config.text.goal;
    let chrome = Chrome {
        title: vec![Span::raw(texts.box_title.clone())],
        hint: Some(texts.box_hint.clone()),
    };
    let goal = seen.talk.transcript.goal.as_ref();
    let spent = goal.and_then(|g| {
        crate::money::amounts(&g.bill, &seen.link.currency, &seen.config.layout.currencies)
    });
    let rows = goal.map_or_else(Vec::new, |g| {
        rows(
            g,
            spent.as_deref(),
            seen.config,
            (width, jiff::Timestamp::now()),
        )
    });
    let lines: Vec<Line<'static>> = rows
        .into_iter()
        .skip(scroll)
        .take(max)
        .map(|line| panel::item(false, line.spans, None, width))
        .collect();
    let map = vec![None; lines.len()];
    (chrome, lines, map)
}

/// 排成的行：名字一列对齐，值折行照值那一列对齐。
pub(crate) fn rows(
    goal: &Goal,
    spent: Option<&str>,
    config: &Config,
    (width, now): (u16, jiff::Timestamp),
) -> Vec<Line<'static>> {
    let t = &config.text.goal;
    let keys = &t.box_keys;
    let state = match goal.state {
        GoalState::Running => &t.running,
        GoalState::Waiting => &t.waiting,
        GoalState::Paused => &t.paused,
        GoalState::Blocked => &t.blocked,
        GoalState::Completed => &t.completed,
        GoalState::Other => &t.some_round,
    };
    let sep = crate::sheet::sep();
    let mut state = match &goal.reason {
        Some(reason) => format!("{state}{sep}{reason}"),
        None => state.clone(),
    };
    // 停了又续上过的接「· 恢复 N 次」，0 次不写（核心 GOAL-1，同网页的说法）。
    if goal.resumes > 0 {
        state.push_str(&sep);
        state.push_str(&t.resumes.replace("{n}", &goal.resumes.to_string()));
    }
    let rounds = match goal.limit {
        Some(limit) => format!("{}/{limit}", goal.round),
        None => goal.round.to_string(),
    };
    let mut items = vec![
        (&keys[0], goal.text.clone()),
        (&keys[1], state),
        (&keys[2], rounds),
    ];
    if let Some(since) = goal.since {
        let secs = super::goal_tag::elapsed(since, now);
        items.push((&keys[3], crate::meter::clock(secs)));
    }
    if goal.tokens > 0 {
        let mut usage = t
            .box_tokens
            .replace("{tokens}", &crate::meter::short(goal.tokens));
        if let Some(spent) = spent {
            usage.push_str(&crate::sheet::sep());
            usage.push_str(spent);
        }
        items.push((&keys[4], usage));
    }
    let name_w = items.iter().map(|(k, _)| k.width()).max().unwrap_or(0);
    let room = usize::from(width).saturating_sub(name_w + 4).max(1);
    let mut out = Vec::new();
    for (key, value) in items {
        let text = value.trim();
        for (i, piece) in wrap_words(text, u16::try_from(room).unwrap_or(u16::MAX))
            .into_iter()
            .enumerate()
        {
            let name = if i == 0 { key.as_str() } else { "" };
            let pad = " ".repeat(name_w - name.width() + 2);
            let seg = text[piece.start..piece.end]
                .trim_end_matches('\n')
                .to_string();
            out.push(Line::from(vec![
                Span::styled(format!("{name}{pad}"), theme::dim()),
                Span::raw(seg),
            ]));
        }
    }
    out
}

#[cfg(test)]
mod tests;
