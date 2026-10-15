//! 一段排成几行（蓝图「界面块」第 6 条）：小标题、字、一对对、进度条、按钮；列表在 `list.rs`。只有字和列表能收（`left`
//! 是还剩几行），别的照全排，放不放得下由外面定。

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::{Drawn, Hit, Look, Mode, list, wrap};
use crate::cells::clip;
use crate::theme;
use crate::widgets::{Button, Pair, Segment};

/// 排一段。
pub(super) fn lay(segment: &Segment, look: &Look, left: Option<usize>) -> Drawn {
    let room = usize::from(look.room());
    match segment {
        Segment::Heading(text) => {
            let style = if look.mode == Mode::Sidebar {
                theme::side_title()
            } else {
                Style::new().add_modifier(Modifier::BOLD)
            };
            lines(vec![Line::styled(clip(text, room), style)])
        }
        Segment::Text { text, tone } => {
            let style = look.tone(*tone);
            let mut pieces = wrap::soft(text, room);
            let cut = left.is_some_and(|l| l > 0 && pieces.len() > l);
            if let Some(l) = left.filter(|_| cut) {
                pieces.truncate(l);
                if let Some(last) = pieces.last_mut() {
                    *last = format!("{}…", wrap::take(last, room.saturating_sub(1)));
                }
            }
            let mut drawn = lines(pieces.into_iter().map(|p| Line::styled(p, style)).collect());
            drawn.cut = cut;
            drawn
        }
        Segment::Pairs(pairs) => lines(pairs_lines(pairs, look, room)),
        Segment::Progress { value, text, mark } => {
            let mut out = Vec::new();
            if let Some(text) = text {
                out.push(Line::styled(clip(text, room), theme::dim()));
            }
            // 没写字的条后面接百分比，给它留五列。
            let tail = text.is_none().then(|| format!(" {:.0}%", value * 100.0));
            let cells = look
                .bar
                .width
                .min(room.saturating_sub(tail.as_ref().map_or(0, |t| t.width())))
                .max(1);
            let mut bar = bar(*value, *mark, cells, look);
            if let Some(tail) = tail {
                bar.spans.push(Span::styled(tail, theme::dim()));
            }
            out.push(bar);
            lines(out)
        }
        Segment::List(items) => list::lay(items, look, left),
        Segment::Buttons(buttons) => buttons_lines(buttons, room),
    }
}

fn lines(lines: Vec<Line<'static>>) -> Drawn {
    Drawn {
        lines,
        hits: Vec::new(),
        cut: false,
    }
}

/// 一对一行：名字暗、值照语气；名字一列对齐，最多占一半宽，长的截掉加 `…`。
fn pairs_lines(pairs: &[Pair], look: &Look, room: usize) -> Vec<Line<'static>> {
    let widest = pairs.iter().map(|p| p.label.width()).max().unwrap_or(0);
    let column = widest.min(room / 2);
    pairs
        .iter()
        .map(|pair| {
            let label = clip(&pair.label, column);
            let pad = " ".repeat(column.saturating_sub(label.width()) + 2);
            let value = clip(&pair.value, room.saturating_sub(column + 2));
            Line::from(vec![
                Span::styled(label, theme::dim()),
                Span::raw(pad),
                Span::styled(value, look.tone(pair.tone)),
            ])
        })
        .collect()
}

/// 一根进度条，画法同侧边栏「上下文」那根：满了的强调色、没满的暗；有过进度就至少亮一格；`mark` 落在的那一格警示色。
fn bar(value: f64, mark: Option<f64>, cells: usize, look: &Look) -> Line<'static> {
    let full = if value > 0.0 {
        ((value * cells as f64).round() as usize).clamp(1, cells)
    } else {
        0
    };
    let mark = mark.map(|m| ((m * cells as f64).floor() as usize).min(cells - 1));
    let spans = (0..cells)
        .map(|i| {
            let (glyph, style) = if i < full {
                (&look.bar.full, theme::accent())
            } else {
                (&look.bar.empty, theme::dim())
            };
            let style = if mark == Some(i) {
                theme::warn()
            } else {
                style
            };
            Span::styled(glyph.clone(), style)
        })
        .collect::<Vec<_>>();
    Line::from(spans)
}

/// 一排按钮：字强调色，按钮之间空两格，放不下的折到下一行；每个按钮记一个点击区。
fn buttons_lines(buttons: &[Button], room: usize) -> Drawn {
    let mut drawn = lines(vec![Line::default()]);
    let mut used = 0;
    for button in buttons {
        let label = clip(&button.label, room);
        let w = label.width();
        let gap = if used > 0 { 2 } else { 0 };
        if used > 0 && used + gap + w > room {
            drawn.lines.push(Line::default());
            used = 0;
        }
        let gap = if used > 0 { 2 } else { 0 };
        let row = drawn.lines.len() - 1;
        if let Some(line) = drawn.lines.last_mut() {
            if gap > 0 {
                line.spans.push(Span::raw("  "));
            }
            line.spans.push(Span::styled(label, theme::accent()));
        }
        let from = u16::try_from(used + gap).unwrap_or(u16::MAX);
        drawn.hits.push(Hit {
            row,
            from,
            to: from.saturating_add(u16::try_from(w).unwrap_or(u16::MAX)),
            action: button.action.clone(),
        });
        used += gap + w;
    }
    drawn
}
