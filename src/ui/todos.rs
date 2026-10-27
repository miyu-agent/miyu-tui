//! 待办那一块（蓝图 `tui.md`「后台命令、子代理和侧边栏」第 4、4a 条）：一律画在输入框上面（2026-10-26 项目主人：侧边栏不放
//! 待办）。收着时只露正在做的那一项和后面两项没做的，标题写做完几项、一共几项；点开全部、长的折行。

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::config::{Config, TodoMarks};
use crate::jobs::{Board, Todo, TodoState};
use crate::theme;

/// 收着时露几项没做的（在做的那一项之外）。
const NEXT: usize = 2;

/// 待办那一块：`待办 2/5`，下面一项一行、一个记号。没有待办、全做完了是空的。`full` 时没做完的照她写的先后在前、做完的在
/// 后，全部列出、长的折行；不然只露正在做的第一项和接着两项没做的（没有在做的露前三项没做的），长的截掉加 `…`。
pub fn todo_lines(board: &Board, config: &Config, width: u16, full: bool) -> Vec<Line<'static>> {
    let words = &config.text.jobs;
    let marks = &config.layout.todo_marks;
    let Some((done, total)) = board.todo_shown() else {
        return Vec::new();
    };
    let title = words
        .todo
        .replace("{done}", &done.to_string())
        .replace("{total}", &total.to_string());
    let mut out = vec![Line::styled(title, theme::dim())];
    let (open, finished): (Vec<&Todo>, Vec<&Todo>) =
        board.todos.iter().partition(|t| t.state != TodoState::Done);
    if full {
        for todo in open.iter().chain(&finished) {
            out.extend(item(todo, marks, width, true));
        }
        return out;
    }
    for todo in shown(&open) {
        out.extend(item(todo, marks, width, false));
    }
    out
}

/// 收着时露的几项：正在做的第一项，接着没做的前两项；没有在做的，没做的前三项。
fn shown<'a>(open: &[&'a Todo]) -> Vec<&'a Todo> {
    let pending = open.iter().filter(|t| t.state == TodoState::Pending);
    match open.iter().find(|t| t.state == TodoState::Active) {
        Some(active) => std::iter::once(*active)
            .chain(pending.take(NEXT).copied())
            .collect(),
        None => pending.take(NEXT + 1).copied().collect(),
    }
}

/// 一项：记号加字。`wrap` 时长的折行、和字对齐，不然截掉加 `…`。
fn item(todo: &Todo, marks: &TodoMarks, width: u16, wrap: bool) -> Vec<Line<'static>> {
    let (mark, style) = match todo.state {
        TodoState::Pending => (&marks.pending, theme::dim()),
        TodoState::Active => (&marks.active, theme::accent().add_modifier(Modifier::BOLD)),
        TodoState::Done => (
            &marks.done,
            theme::dim().add_modifier(Modifier::CROSSED_OUT),
        ),
    };
    let room = usize::from(width).saturating_sub(mark.width()).max(1);
    let lead_style = style.remove_modifier(Modifier::CROSSED_OUT);
    if !wrap {
        let text = super::rows::clip(&todo.text, u16::try_from(room).unwrap_or(1));
        return vec![Line::from(vec![
            Span::styled(mark.clone(), lead_style),
            Span::styled(text, style),
        ])];
    }
    let indent = " ".repeat(mark.width());
    crate::cells::wrap(&todo.text, room)
        .into_iter()
        .enumerate()
        .map(|(i, piece)| {
            let (lead, lead_style) = if i == 0 {
                (mark.clone(), lead_style)
            } else {
                (indent.clone(), Style::new())
            };
            Line::from(vec![
                Span::styled(lead, lead_style),
                Span::styled(piece, style),
            ])
        })
        .collect()
}

#[cfg(test)]
mod tests;
