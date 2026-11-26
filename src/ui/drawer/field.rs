//! 抽屉里写字的那几行（2026-11-27 项目主人：光标挪不了、换行看不出来、长的只留末尾一截）：照输入框的画法画
//! [`Field`]（粘贴块、选区的样子同输入框，`ui/input_box.rs` 的 `styled_line`），放不下的软换行，长高到上限以后在里面滚，
//! 光标所在的那一行一定露着；空的写暗色的提示，光标停在提示的第一个字上。写完以后（不在编辑）那一项的字照 [`settled`] 折。

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::items::folded;
use crate::input::Field;
use crate::theme;

/// 写完以后（不在编辑）那一项的字最多折几行，再多的最后一行接 `…`：一项不占掉整个抽屉。
pub const SETTLED_ROWS: usize = 3;

/// 写字的那一块在排好的行里在哪：第几行起、从第几列、多宽、露几行、露的第一行是折好的第几行。画的时候换成屏幕上的位置
/// 记回 [`Field`]，鼠标点一下照它放光标。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Spot {
    /// 第一行在排好的行里是第几行。
    pub row: usize,
    /// 字从第几列起。
    pub col: u16,
    /// 折行的宽。
    pub width: u16,
    /// 露几行。
    pub rows: usize,
    /// 露的第一行是折好的第几行。
    pub top: usize,
    /// 抽屉放不下、头几行被裁掉了几行（`fit`）：屏幕上第一行是折好的第 `top + skip` 行。
    pub skip: usize,
}

/// 排好的写字处：每一行的字（不带前面的缩进）、光标在第几行第几列（从这一块的第一行、字的第一列数）、露的第一行是第几行。
pub struct Rows {
    /// 每一行。
    pub lines: Vec<Vec<Span<'static>>>,
    /// 光标。
    pub cursor: (usize, u16),
    /// 露的第一行是折好的第几行。
    pub top: usize,
}

/// 照 `width` 列折、最多露 `cap` 行排好 `field`；空的写暗色的 `hint`（截到 `width` 列）。
pub fn rows(field: &Field, width: u16, cap: usize, hint: &str) -> Rows {
    if field.text().is_empty() {
        let hint = crate::cells::clip(hint, usize::from(width));
        let lines = vec![if hint.is_empty() {
            Vec::new()
        } else {
            vec![Span::styled(hint, theme::dim())]
        }];
        return Rows {
            lines,
            cursor: (0, 0),
            top: 0,
        };
    }
    let shown = field.shown(width, cap);
    let editor = field.editor();
    let blocks = editor.blocks();
    let lines = shown
        .lines
        .iter()
        .map(|l| {
            crate::ui::input_box::styled_line(editor.text(), *l, editor.selection(), &blocks, None)
                .spans
                .into_iter()
                .map(|s| Span::styled(s.content.into_owned(), s.style))
                .collect()
        })
        .collect();
    let (row, col) = shown.cursor;
    Rows {
        lines,
        cursor: (row, u16::try_from(col).unwrap_or(u16::MAX)),
        top: shown.top,
    }
}

/// 写完以后（不在编辑）那一项写的字：去掉前后的空白，照原文的换行分行，长的照 `room` 列折（同选项的标题），最多
/// [`SETTLED_ROWS`] 行，再多的最后一行接 `…`（2026-11-27：原来换行写成 `↵` 挤在一行里）。
pub fn settled(text: &str, room: u16) -> Vec<String> {
    let mut rows: Vec<String> = text
        .trim()
        .split('\n')
        .flat_map(|l| folded(l, room))
        .collect();
    if rows.len() > SETTLED_ROWS {
        rows.truncate(SETTLED_ROWS);
        let mark = crate::sheet::ellipsis();
        let last = rows.pop().unwrap_or_default();
        let joined = format!("{}{mark}", last.trim_end());
        // 放不下 `…` 的截掉后面几个字（截的地方本来就写 `…`，不重复）。
        let fit = if joined.width() <= usize::from(room) {
            joined
        } else {
            crate::cells::clip(&joined, usize::from(room))
        };
        rows.push(fit);
    }
    rows
}

/// 一行：前面 `lead` 列的空白，接着这一行的字。
pub fn indented(lead: usize, spans: Vec<Span<'static>>) -> Line<'static> {
    let mut all = vec![Span::raw(" ".repeat(lead))];
    all.extend(spans);
    Line::from(all)
}
