//! 她答话时你敲的字（2026-10-22 项目主人：「回车当插话吧」）：活动区最下面先画排着的插话（暗色，同全屏的排队），再画
//! 提前敲的那一行，光标停在行尾、输入法的候选框跟着它。回车插话、Ctrl+回车打断见 `turn/interject.rs`。

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::turn::Turn;
use crate::config::Config;

/// 排着的插话、输入那一行，交回光标停在最后一行的第几列。输入那一行她一开始答就留着（光标亮着停在那里，输入法的候选框
/// 一开始就在对的地方，2026-10-23 项目主人；为什么亮着见 `paint.rs` 的 `Caret`）；没打字时什么都不画，打了字画指示符和字。
/// 抽屉开着时（`typing` 是假）只画排着的。
pub fn lines(
    turn: &Turn,
    config: &Config,
    width: u16,
    typing: bool,
) -> (Vec<Line<'static>>, Option<u16>) {
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
    let mut col = None;
    if typing {
        let pointer = &config.text.drawer.pointer;
        let room = width.saturating_sub(pointer.width() + 1);
        let shown = tail(&one_line(&turn.typed), room);
        col = u16::try_from(2 + pointer.width() + shown.width()).ok();
        out.push(if !shown.is_empty() {
            Line::from(vec![
                Span::raw("  "),
                Span::styled(pointer.clone(), crate::theme::accent()),
                Span::raw(shown),
            ])
        } else {
            Line::default()
        });
    }
    // 和上面她的回答空一行。
    if !out.is_empty() {
        out.insert(0, Line::default());
    }
    (out, col)
}

/// 换行写成空格：一条一行。
fn one_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ")
}

/// 放不下的只留后面那一截（光标在最后，看得到正在敲的字）。
fn tail(text: &str, room: usize) -> String {
    let mut out: Vec<char> = Vec::new();
    let mut used = 0;
    for c in text.chars().rev() {
        let w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if used + w > room {
            break;
        }
        used += w;
        out.push(c);
    }
    out.into_iter().rev().collect()
}
