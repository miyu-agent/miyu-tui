//! 一行一根小条（发版前 check 32，2026-11-09 项目主人从三个样板里挑的「每行一根小条」，像磁盘占用）：名字、按占比长短的条、
//! 写短的数、百分比，四列各自对齐。上下文的各块照它排。

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::{pad_left, pad_right};
use crate::meter;
use crate::theme;

/// 条最宽几格；框窄时照剩下的缩，缩到 [`MIN`] 以下就不画条。
pub(super) const WIDTH: usize = 16;
const MIN: usize = 4;

/// 一行：缩进几层、名字、多少。
pub(super) type Row = (usize, String, u64);

/// 排好的几行和它们一共多宽（段头右边那一格照它收尾）。
pub(super) struct Laid {
    pub lines: Vec<Line<'static>>,
    pub width: usize,
}

/// `whole` 里占 `part` 的条：`width` 格，八分之一格一档；不是 0 的至少画八分之一格，看得出有。
pub(super) fn bar(part: u64, whole: u64, width: usize) -> String {
    let total = width as u64 * 8;
    let eighths = if whole == 0 {
        0
    } else {
        ((part as f64 / whole as f64) * total as f64).round() as u64
    };
    let eighths = eighths.clamp(u64::from(part > 0), total);
    // 一格里八分之一到八分之八宽（主题的 `marks.fill`）。
    let marks = crate::sheet::read(|s| s.marks.fill.clone());
    let full = (eighths / 8) as usize;
    let rest = match eighths % 8 {
        0 => "",
        n => marks[n as usize - 1].as_str(),
    };
    let drawn = format!("{}{rest}", marks[7].repeat(full));
    pad_right(&drawn, width)
}

/// 一行一根条：第 0 层名字正常色、条照主题的强调色；子块缩进两格，名字、数、条都暗。百分比占 `whole`。
pub(super) fn rows(list: &[Row], whole: u64, room: usize) -> Laid {
    let name_width = list
        .iter()
        .map(|(depth, label, _)| depth * 2 + label.width())
        .max()
        .unwrap_or(0)
        + 2;
    let numbers: Vec<String> = list.iter().map(|(_, _, n)| meter::short(*n)).collect();
    let number_width = numbers.iter().map(|n| n.width()).max().unwrap_or(0) + 1;
    let share_width = 7;
    let fixed = name_width + number_width + share_width;
    let bar_width = WIDTH.min(room.saturating_sub(fixed));
    let bar_width = if bar_width < MIN { 0 } else { bar_width };
    let lines = list
        .iter()
        .zip(&numbers)
        .map(|((depth, label, tokens), number)| {
            let (style, fill) = match depth {
                0 => (Style::default(), theme::accent()),
                _ => (theme::dim(), theme::dim()),
            };
            let name = pad_right(&format!("{}{label}", "  ".repeat(*depth)), name_width);
            let share = format!("{}%", meter::percent_tenths(*tokens, whole));
            let mut spans = vec![Span::styled(name, style)];
            if bar_width > 0 {
                // 小块不画条（2026-11-10 项目主人：「暗色的明细我觉得不需要条」），空着占位，数照旧对齐。
                let drawn = match depth {
                    0 => bar(*tokens, whole, bar_width),
                    _ => " ".repeat(bar_width),
                };
                spans.push(Span::styled(drawn, fill));
            }
            spans.push(Span::styled(pad_left(number, number_width), style));
            spans.push(Span::styled(pad_left(&share, share_width), theme::dim()));
            Line::from(spans)
        })
        .collect();
    Laid {
        lines,
        width: fixed + bar_width,
    }
}

/// 段头：左边加粗，右边正常色收尾在 `width` 那一列（和下面的数一列，发版前 check 第 19 条）；放不下的空两格接着写。
pub(super) fn head(left: &str, right: &str, width: usize) -> Line<'static> {
    let gap = width.saturating_sub(left.width() + right.width()).max(2);
    Line::from(vec![
        Span::styled(
            left.to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ".repeat(gap)),
        Span::raw(right.to_string()),
    ])
}
