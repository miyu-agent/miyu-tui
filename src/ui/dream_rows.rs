//! 人打的 `/dream` 整理时接在正文末尾的那一块（蓝图 `tui.md`「整理记忆」第 3 到 6 条）：第一行第几批，5 行滚动区，空一行，
//! 两行夜空（左边吉祥物睡觉冒 z，右边星星散着一颗颗亮）。只给记号上色，字用正文色或暗色。

use std::collections::HashMap;
use std::f64::consts::TAU;
use std::time::Instant;

use ratatui::style::Modifier;
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

use super::rows::{Ctx, Row};
use crate::config::{DreamLook, DreamTexts};
use crate::dream::{Dream, Line, ROWS, Retire, place};
use crate::theme;

/// 排成的行。
pub fn rows(dream: &Dream, ctx: &Ctx, now: Instant) -> Vec<Row> {
    let look = &ctx.config.layout.dream;
    let texts = &ctx.config.text.dream;
    let width = usize::from(ctx.width);
    let mut head = vec![
        Span::styled(look.mark.clone(), theme::warn()),
        Span::raw(texts.title.clone()),
    ];
    if let Some((n, total)) = dream.batch() {
        let batch = texts
            .batch
            .replace("{n}", &n.to_string())
            .replace("{total}", &total.to_string());
        head.push(Span::styled(batch, theme::dim()));
    }
    // 和上面的正文空一行，同等第一个字时的转圈（「时间线」第 19 条）。
    let mut out = vec![
        ctx.row(ctx.blank_slot(), Vec::new()),
        ctx.row(ctx.blank_slot(), head),
    ];
    let lines: Vec<&Line> = dream.rows().collect();
    for _ in lines.len()..ROWS {
        out.push(ctx.row(ctx.blank_slot(), Vec::new()));
    }
    for line in lines {
        out.push(ctx.row(ctx.blank_slot(), line_spans(line, width, look, texts)));
    }
    out.push(ctx.row(ctx.blank_slot(), Vec::new()));
    let seconds = now.saturating_duration_since(dream.started).as_secs_f64();
    for spans in sky(dream, width, seconds, look, texts) {
        out.push(ctx.row(ctx.blank_slot(), spans));
    }
    out
}

/// 滚动区的一行：一条一行，宽了截掉加 `…`，不折行（第 4 条）。
fn line_spans(
    line: &Line,
    width: usize,
    look: &DreamLook,
    texts: &DreamTexts,
) -> Vec<Span<'static>> {
    let room = |mark: &str, tail: &str| width.saturating_sub(mark.width() + tail.width());
    match line {
        Line::Float(text) => {
            let text = format!("{}{text}", look.float);
            vec![Span::styled(crate::cells::clip(&text, width), theme::dim())]
        }
        Line::Retired(text, why) => {
            let why = match why {
                Retire::Outdated => &texts.outdated,
                Retire::Repeat => &texts.repeat,
                Retire::Worthless => &texts.worthless,
            };
            let tail = format!("{}{why}", look.reason_gap);
            let text = crate::cells::clip(text, room(&look.retired, &tail));
            vec![
                Span::styled(look.retired.clone(), theme::removed()),
                Span::styled(text, theme::dim().add_modifier(Modifier::CROSSED_OUT)),
                Span::styled(tail, theme::dim()),
            ]
        }
        Line::Revised(text) => vec![
            Span::styled(look.revised.clone(), theme::accent()),
            Span::raw(crate::cells::clip(text, room(&look.revised, ""))),
        ],
        Line::Summary(chars, done) => {
            let mut text = texts.summary.replace("{chars}", &chars.to_string());
            if !done {
                text.push_str(&texts.writing);
            }
            vec![
                Span::styled(look.summary.clone(), theme::good()),
                Span::styled(
                    crate::cells::clip(&text, room(&look.summary, "")),
                    theme::dim(),
                ),
            ]
        }
    }
}

/// 两行夜空（第 5、6 条）：左边一栏吉祥物和 z，右边星星，右下角合了多少。
fn sky(
    dream: &Dream,
    width: usize,
    seconds: f64,
    look: &DreamLook,
    texts: &DreamTexts,
) -> [Vec<Span<'static>>; 2] {
    let woke = dream.woke.is_some();
    let frame = (seconds * 1000.0 / look.z_frame_ms.max(1) as f64) as usize;
    let pick = |frames: &[String]| {
        frames
            .get(frame % frames.len().max(1))
            .cloned()
            .unwrap_or_default()
    };
    let (top, face, side) = if woke {
        (
            format!("    {}", look.wake_mark),
            &look.awake,
            String::new(),
        )
    } else {
        (pick(&look.z_top), &look.asleep, pick(&look.z_side))
    };
    let pad = |text: String| {
        let gap = look.left_width.saturating_sub(text.width());
        format!("{text}{}", " ".repeat(gap))
    };
    // 左栏第二行：脸和它右边那一截一起补到左栏宽。
    let face = format!("  {face}");
    let side = crate::cells::clip(&side, look.left_width.saturating_sub(face.width()));
    let rest = look.left_width.saturating_sub(face.width() + side.width());
    let mut rows = [
        vec![Span::styled(
            pad(top),
            if woke { theme::warn() } else { theme::dim() },
        )],
        vec![
            Span::raw(face),
            Span::styled(side, theme::dim()),
            Span::raw(" ".repeat(rest)),
        ],
    ];
    let (merged, until, total) = dream.progress();
    let tally = (total > 0).then(|| {
        texts
            .tally
            .replace("{merged}", &merged.min(total).to_string())
            .replace("{total}", &total.to_string())
    });
    let tally_width = tally.as_ref().map_or(0, |t| t.width() + 2);
    let field = width.saturating_sub(look.left_width + tally_width);
    let count = look.stars.min(field / look.star_cols.max(1));
    let spots = place(field, count, look.glyphs.len(), look.seed);
    let per = if count == 0 {
        1.0
    } else {
        total.max(1) as f64 / count as f64
    };
    let lit = if woke {
        count
    } else {
        (merged as f64 / per).floor() as usize
    };
    let soon = (until as f64 / per).ceil() as usize;
    let at: HashMap<(usize, usize), usize> = spots
        .iter()
        .enumerate()
        .map(|(i, s)| ((s.row, s.col), i))
        .collect();
    for (row, spans) in rows.iter_mut().enumerate() {
        let mut gap = String::new();
        for col in 0..field {
            let Some(&i) = at.get(&(row, col)) else {
                gap.push(' ');
                continue;
            };
            if !gap.is_empty() {
                spans.push(Span::raw(std::mem::take(&mut gap)));
            }
            let spot = spots[i];
            spans.push(if spot.rank < lit {
                let glyph = look.glyphs.get(spot.glyph).cloned().unwrap_or_default();
                let twinkles = woke || i % look.twinkle_every.max(1) == 0;
                let lift = if twinkles {
                    twinkle(seconds, i, look)
                } else {
                    0.0
                };
                Span::styled(glyph, theme::lifted(theme::warn(), lift))
            } else if spot.rank < soon {
                Span::styled(look.soon.clone(), theme::warn().add_modifier(Modifier::DIM))
            } else {
                Span::styled(look.unlit.clone(), theme::faint())
            });
        }
        if row == 1
            && let Some(tally) = &tally
        {
            gap.push_str("  ");
            spans.push(Span::raw(gap));
            spans.push(Span::styled(tally.clone(), theme::dim()));
        }
    }
    rows
}

/// 亮了的星闪一下往白里偏多少：大半时间不动，偶尔一亮（照 `twinkle_ms` 一个来回，各颗错开）。
fn twinkle(seconds: f64, i: usize, look: &DreamLook) -> f64 {
    let period = look.twinkle_ms.max(1) as f64 / 1000.0;
    let phase = seconds / period + i as f64 * 0.37;
    let wave = (phase * TAU).sin().max(0.0);
    wave.powi(4) * look.twinkle_lift
}

#[cfg(test)]
mod tests;
