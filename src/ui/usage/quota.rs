//! 总览顶上的套餐额度（核心 QU-1，蓝图「配置与模型」第 5 条，2026-11-14 项目主人定样子）：一行一个窗口，名字、带框的条、
//! 百分比、重置时刻同一行，`5 小时  ▕██                  ▏  4%  2 小时后重置`；用到八成变黄、满了变红；24 小时内写
//! 「x 小时后重置」（不到一小时写分钟），再远写日期时刻。没有额度的一行都不画。条和两边的记号照主题（`marks.block`、
//! `marks.edge`），纯 ASCII 主题是 `[##        ]`。

use jiff::Timestamp;
use jiff::tz::TimeZone;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::Texts;
use crate::app::quota::QuotaWindow;
use crate::theme;

/// 条最长几格（主人给的样子是 20 格）。
const CELLS: usize = 20;

/// 条最短几格：再窄就不写重置时刻。
const MIN_CELLS: usize = 6;

/// 用到几成变黄。
const WARN_AT: f64 = 80.0;

/// 条用的记号：填满的一格、左右两边（主题的 `marks.block`、`marks.edge`）。
pub(super) struct Glyphs<'a> {
    pub block: &'a str,
    pub edge: &'a [String; 2],
}

/// 额度那几行；没有额度的是空的。
pub(super) fn lines(
    texts: &Texts,
    windows: &[QuotaWindow],
    (now, tz): (Timestamp, &TimeZone),
    width: u16,
    glyphs: &Glyphs<'_>,
) -> Vec<Line<'static>> {
    let name_w = windows.iter().map(|w| w.label.width()).max().unwrap_or(0);
    let width = usize::from(width);
    let resets: Vec<Option<String>> = windows
        .iter()
        .map(|w| w.resets_at.map(|at| reset_text(texts, at, now, tz)))
        .collect();
    // 几行排成一张表：条一样长，照最宽的那个重置时刻留地方（2026-11-17 验收第 36 条：右边的字宽窄不同时条不能跟着变短）。
    // 名字、两格、条（两边框各一格）、两格、百分比（四格）；放得下再接两格和重置时刻，放不下的几行都不写。
    let fixed = name_w + 2 + 2 + 2 + 4;
    let widest = resets.iter().flatten().map(|r| 2 + r.width()).max();
    let room = |extra: usize| width.saturating_sub(fixed + extra);
    let (cells, with_reset) = match widest {
        Some(extra) if room(extra) >= MIN_CELLS => (room(extra).min(CELLS), true),
        _ => (room(0).clamp(1, CELLS), false),
    };
    windows
        .iter()
        .zip(resets)
        .map(|(w, reset)| {
            let percent = format!("{:>4}", format!("{}%", w.percent.round() as i64));
            let reset = reset.filter(|_| with_reset);
            let style = level(w.percent);
            let pad = name_w - w.label.width();
            let mut spans = vec![
                Span::raw(format!("{}{}  ", w.label, " ".repeat(pad))),
                Span::styled(glyphs.edge[0].clone(), theme::dim()),
                Span::styled(fill(w.percent, cells, glyphs.block), style),
                Span::styled(glyphs.edge[1].clone(), theme::dim()),
                Span::styled(format!("  {percent}"), style),
            ];
            if let Some(reset) = reset {
                spans.push(Span::styled(format!("  {reset}"), theme::dim()));
            }
            Line::from(spans)
        })
        .collect()
}

/// 用了多少照什么色：八成以上黄、满了红，别的照平常的字。
fn level(percent: f64) -> Style {
    if percent >= 100.0 {
        theme::error()
    } else if percent >= WARN_AT {
        theme::warn()
    } else {
        Style::new()
    }
}

/// 条里面 `cells` 格：用了的那几格填满，剩下的空着；用了一点点的也至少画一格。
fn fill(percent: f64, cells: usize, block: &str) -> String {
    let ratio = (percent / 100.0).clamp(0.0, 1.0);
    let mut full = (ratio * cells as f64).round() as usize;
    if percent > 0.0 && full == 0 {
        full = 1;
    }
    format!("{}{}", block.repeat(full), " ".repeat(cells - full))
}

/// 重置时刻：24 小时内写「x 小时后重置」，不到一小时写分钟；再远写本地的日期时刻；已经过了的写「0 分钟后」。
fn reset_text(texts: &Texts, at: Timestamp, now: Timestamp, tz: &TimeZone) -> String {
    let secs = (at.as_second() - now.as_second()).max(0);
    if secs < 3600 {
        let minutes = (secs + 59) / 60;
        texts.quota_minutes.replace("{n}", &minutes.to_string())
    } else if secs < 24 * 3600 {
        let hours = (secs + 1800) / 3600;
        texts.quota_hours.replace("{n}", &hours.to_string())
    } else {
        let time = at.to_zoned(tz.clone()).strftime("%m-%d %H:%M").to_string();
        texts.quota_at.replace("{time}", &time)
    }
}

#[cfg(test)]
mod tests;
