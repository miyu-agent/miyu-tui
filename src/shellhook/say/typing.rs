//! 她答话时你敲的字（2026-10-22 项目主人：「回车当插话吧」）：活动区最下面先画排着的插话（暗色，同全屏的排队），再画
//! 提前敲的那几行（照全屏输入框排，排期第 46 条，`ahead.rs`），光标停在打字的那一格、输入法的候选框跟着它。回车插话、Ctrl+回车
//! 打断见 `turn/interject.rs`。

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::turn::Turn;
use crate::config::Config;

/// 排着的插话、输入那几行，交回光标在倒数第几行（0 是最后一行）、第几列。输入那一行她一开始答就留着（光标亮着停在那里，输入法的候选框
/// 一开始就在对的地方，2026-10-23 项目主人；为什么亮着见 `paint.rs` 的 `Caret`）；没打字时什么都不画，打了字画指示符和字。
/// 抽屉开着时（`typing` 是假）只画排着的。
pub fn lines(
    turn: &Turn,
    config: &Config,
    width: u16,
    typing: bool,
) -> (Vec<Line<'static>>, Option<(usize, u16)>) {
    let width = usize::from(width).saturating_sub(2);
    let mut out: Vec<Line<'static>> = turn
        .transcript
        .entries
        .iter()
        .filter(|e| e.queued && turn.interjected(e.id))
        .map(|e| {
            let text = format!("{}{}", config.layout.queued_mark, one_line(&e.text));
            Line::styled(
                format!("  {}", crate::cells::clip(&text, width)),
                crate::theme::dim(),
            )
        })
        .collect();
    let mut caret = None;
    if typing {
        // 照全屏输入框排（`ahead.rs`）：折行、块的样子同全屏，第一行打头指示符，后面几行缩进对齐；光标停在打字的那一格。
        let pointer = &config.text.drawer.pointer;
        let lead = 2 + pointer.width();
        let room = u16::try_from(width.saturating_sub(lead + 1)).unwrap_or(u16::MAX);
        let (rows, (row, col)) = turn.typed.rows(room, usize::from(config.layout.max_rows));
        if !turn.typed.is_empty() {
            let below = rows.len().saturating_sub(row + 1);
            caret = u16::try_from(lead + col).ok().map(|col| (below, col));
            for (i, line) in rows.into_iter().enumerate() {
                let mut spans = vec![Span::raw("  ")];
                spans.push(if i == 0 {
                    Span::styled(pointer.clone(), crate::theme::accent())
                } else {
                    Span::raw(" ".repeat(pointer.width()))
                });
                spans.extend(line.spans);
                out.push(Line::from(spans));
            }
        } else {
            caret = Some((0, u16::try_from(lead).unwrap_or(0)));
            out.push(Line::default());
        }
    }
    // 和上面她的回答空一行。
    if !out.is_empty() {
        out.insert(0, Line::default());
    }
    (out, caret)
}

/// 换行写成空格：一条一行。
fn one_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ")
}
