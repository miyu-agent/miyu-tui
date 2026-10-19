//! 人打的 `/dream` 整理完的回执（蓝图 `tui.md`「整理记忆」第 6 条，2026-10-20 项目主人验收时定）：暗色 `✦` 打头、整行暗色的
//! 一行，照核心的回执写；有改、删、摘要的能点开（同后台任务结束那一行）：整块铺底色，下面列日志里真正存下的，写法同整理时
//! 的滚动区（删红 `✗`、原文删除线和种类，改 `✎` 强调色、新正文，摘要 `✦` 绿、全文），长的折行、不截。

use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

use super::rows::{Ctx, Row, Target, clip};
use crate::dream::{Kept, Retire};
use crate::input::pieces;
use crate::theme;
use crate::transcript::Entry;

/// 排成的行。`i` 是它在正文里是第几条。
pub fn rows(i: usize, entry: &Entry, ctx: &Ctx) -> Vec<Row> {
    let look = &ctx.config.layout.dream;
    let target = Target::Entry(i);
    let can_open = !entry.kept.is_empty();
    let lit = |s: Style| {
        if can_open && ctx.hover == Some(target) {
            s.add_modifier(Modifier::BOLD)
        } else {
            s
        }
    };
    let width = usize::from(ctx.width).saturating_sub(look.receipt.width());
    let mut head = ctx.row(
        ctx.blank_slot(),
        vec![
            Span::styled(look.receipt.clone(), theme::dim()),
            Span::styled(
                clip(&entry.text, u16::try_from(width).unwrap_or(u16::MAX)),
                lit(theme::dim()),
            ),
        ],
    );
    if !can_open {
        return vec![head];
    }
    head.target = Some(target);
    let mut out = vec![head];
    if entry.open {
        let blank = || ctx.row(ctx.blank_slot(), Vec::new());
        out.push(blank());
        for kept in &entry.kept {
            out.extend(kept_rows(kept, ctx));
        }
        out.push(blank());
        // 点开整块铺底色，连那一行一起（照后台任务结束那一行）。
        for row in &mut out {
            row.shade = true;
            row.target = Some(target);
        }
    }
    out
}

/// 存下的一样：记号打头，字长了折行，折下来的和字对齐。
fn kept_rows(kept: &Kept, ctx: &Ctx) -> Vec<Row> {
    let look = &ctx.config.layout.dream;
    let texts = &ctx.config.text.dream;
    let (mark, mark_style, body, body_style, tail) = match kept {
        Kept::Revised(text) => (
            &look.revised,
            theme::accent(),
            text.clone(),
            Style::new(),
            String::new(),
        ),
        Kept::Retired(text, why) => {
            let why = match why {
                Retire::Outdated => &texts.outdated,
                Retire::Repeat => &texts.repeat,
                Retire::Worthless => &texts.worthless,
            };
            let tail = format!("{}{why}", look.reason_gap);
            (
                &look.retired,
                theme::removed(),
                text.clone(),
                theme::dim().add_modifier(Modifier::CROSSED_OUT),
                tail,
            )
        }
        Kept::Summary(text) => (
            &look.summary,
            theme::good(),
            format!("{}{text}", texts.summary_full),
            theme::dim(),
            String::new(),
        ),
    };
    let lead = mark.width() + 2;
    let width = u16::try_from(usize::from(ctx.width).saturating_sub(lead).max(1)).unwrap_or(1);
    let mut out = Vec::new();
    let pieces = pieces(&body, width);
    let last = pieces.len().saturating_sub(1);
    for (n, (piece, joined)) in pieces.into_iter().enumerate() {
        let first = n == 0;
        let lead = vec![
            Span::raw("  "),
            if first {
                Span::styled(mark.clone(), mark_style)
            } else {
                Span::raw(" ".repeat(mark.width()))
            },
        ];
        let mut content = vec![Span::styled(piece, body_style)];
        if n == last && !tail.is_empty() {
            content.push(Span::styled(tail.clone(), theme::dim()));
        }
        let mut row = ctx.led_row(ctx.blank_slot(), lead, content);
        row.joined = joined;
        out.push(row);
    }
    out
}

#[cfg(test)]
mod tests;
