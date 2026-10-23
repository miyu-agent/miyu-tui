//! 画抽屉（蓝图 `tui.md`「确认和提问的抽屉」第 2、3 条，样子照旧版的提问面板）：画在输入框里，框往上长；
//! 放不下时问题钉在上面、按键提示钉在下面，中间的选项跟着选中的那一项滚。选项的写法在 `items.rs`，
//! 右边的文字画在 `preview.rs`。

mod command;
mod items;
mod preview;

pub use preview::Look as PreviewLook;

#[cfg(test)]
mod tests;

use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

use super::rows::clip;
use crate::caret::Caret;
use crate::drawer::{COMMAND_ROWS, Drawer, Edit, Texts};
use crate::input::pieces;
use crate::theme;

/// 排好的抽屉：每一行、每一行是第几项、选中那一项从第几行起、打字的光标在哪（行、列）。
#[derive(Debug, Default)]
pub struct View {
    /// 每一行。
    pub lines: Vec<Line<'static>>,
    /// 每一行是第几项；不是项的是 `None`。
    pub items: Vec<Option<usize>>,
    /// 选中那一项的第一行、最后一行。
    pub focus: (usize, usize),
    /// 编辑时光标的行、列。
    pub cursor: Option<(usize, u16)>,
    /// 选项前面有几行（谁在问、标签、问题、空行）：放不下时钉在上面。
    pub head: usize,
    /// 前面那几行里放不下时也不裁的：谁在问、标签、问题行（2026-10-22：长命令把问题顶没了）。
    pub pinned: usize,
}

impl View {
    fn push(&mut self, line: Line<'static>, item: Option<usize>) {
        self.lines.push(line);
        self.items.push(item);
    }
}

/// 排成的全部行（还没按高度裁）。
/// `vim`：按键模式是 Vim，按键提示写「j/k」（蓝图「按键」表「按键模式」那一行）。
pub fn view(d: &Drawer, texts: &Texts, look: &PreviewLook, width: u16, vim: bool) -> View {
    let mut v = View::default();
    if let Some(who) = &d.who {
        let who = texts.asking.replace("{who}", who);
        v.push(Line::styled(clip(&who, width), theme::dim()), None);
    }
    if d.has_review() {
        v.push(tabs(d, texts), None);
        v.push(Line::default(), None);
    }
    if !d.on_review() {
        head(d, texts, width, &mut v);
        v.push(Line::default(), None);
    }
    v.head = v.lines.len();
    v.focus = (v.head, v.head);
    if d.on_review() {
        items::review(d, texts, width, &mut v);
    } else {
        preview::body(d, texts, look, width, &mut v);
        items::notes(d, texts, width, &mut v);
    }
    v.push(Line::default(), None);
    // 按过一下 `Esc`：这一行换成黄色的「再按一次 Esc 取消」（第 3、5 条）。
    let last = if d.armed.is_some() {
        Line::styled(clip(&texts.cancel_hint, width), theme::warn())
    } else {
        let hint = super::panel::moves(&keys(d, texts), vim);
        Line::styled(clip(&hint, width), theme::dim())
    };
    v.push(last, None);
    v
}

/// 最后一行按键提示：照这一页能做什么写；编辑时只写保存、退出编辑。
fn keys(d: &Drawer, texts: &Texts) -> String {
    // 写「不允许」的理由不换提示：方向键、两下 Esc 照旧（第 4 条）。
    if matches!(d.editing, Some(Edit::Other | Edit::Notes)) {
        return texts.keys_edit.clone();
    }
    let mut keys = if d.on_review() {
        texts.keys_review.clone()
    } else if !d.is_question() {
        texts.keys_approve.clone()
    } else if d.multiple(d.tab) {
        texts.keys_multi.clone()
    } else {
        texts.keys.clone()
    };
    if d.has_review() {
        keys.push_str(&texts.keys_tabs);
    }
    keys
}

/// 顶上那一排标签，最后一个是「确认」：当前那个反色，答过的后面加 `✓`，别的暗。
fn tabs(d: &Drawer, texts: &Texts) -> Line<'static> {
    let mut spans = Vec::new();
    for page in 0..=d.pages() {
        if page > 0 {
            spans.push(Span::raw("  "));
        }
        let mut label = if page == d.pages() {
            texts.review_tab.clone()
        } else {
            d.label(page)
        };
        if d.answers.get(page).is_some_and(Option::is_some) {
            label.push_str(&texts.answered_tab);
        }
        if page == d.tab {
            let on = Style::new().add_modifier(Modifier::REVERSED);
            spans.push(Span::styled(format!(" {label} "), on));
        } else {
            spans.push(Span::styled(label, theme::dim()));
        }
    }
    Line::from(spans)
}

/// 问题：提问的加粗、放不下折行；确认的问题行加粗，下面装包的几行、每个路径一行暗色，工作区外的标出来。
fn head(d: &Drawer, texts: &Texts, width: u16, v: &mut View) {
    let bold = Style::new().add_modifier(Modifier::BOLD);
    let title = match d.question_at(d.tab) {
        Some(q) => q.question.trim().to_string(),
        None => d.title(texts),
    };
    for (piece, _) in pieces(&title, width) {
        v.push(Line::styled(piece, bold), None);
    }
    // 认成高危命令的（核心 5-17 的 `detail.hazard`）：命令上面红字写一行「高危命令」（2026-10-20 项目主人定）。
    if d.hazardous() {
        v.push(Line::styled(texts.hazard.clone(), theme::error()), None);
    }
    // 这一次不在沙盒里跑（核心 D-4 的 `detail.sandbox` 是 `false`）：黄字写一行，紧跟在问题下面，命令那一块之上。
    if d.unsandboxed() {
        v.push(Line::styled(texts.unsandboxed.clone(), theme::warn()), None);
    }
    // 确认的问题行和这两行提示放不下时也留着（提问的照旧留问题的后半截；2026-10-22 项目主人截图：长命令把它们都顶掉了，
    // 只剩「沙盒外运行」，看着像权限没切过去）。
    if !d.is_question() {
        v.pinned = v.lines.len();
    }
    // 装、卸软件包，打开扩展的：包含、读不成、需要、一并删除、大小，一样一行（第 3 条，2026-10-12 项目主人定）。
    for row in d
        .package_plan(texts)
        .map(|plan| plan.rows)
        .unwrap_or_default()
    {
        for (piece, _) in pieces(&row, width) {
            v.push(Line::styled(piece, theme::dim()), None);
        }
    }
    for (path, mark) in d.paths(texts) {
        // 标记不截：路径长的先截路径，给后面的空格和标记留出地方（原来整行宽截路径，长路径把标记挤没了）。
        let room = mark.as_deref().map_or(0, |m| m.width() + 1);
        let room = width.saturating_sub(u16::try_from(room).unwrap_or(u16::MAX));
        let mut spans = vec![Span::styled(clip(&path, room), theme::dim())];
        if let Some(mark) = mark {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(mark, theme::warn()));
        }
        v.push(Line::from(spans), None);
    }
    // 要跑的命令原文（2026-10-07 项目主人：确认时看不到具体的命令）：空一行，自成一块，长的能滚（`command.rs`）。
    command::push(d, texts, width, v);
}

/// 抽屉要几行：排出来的行数，最多 `max`。
pub fn rows(d: &Drawer, texts: &Texts, look: &PreviewLook, width: u16, max: u16) -> u16 {
    // 按键提示只有一行，换不换「j/k」行数一样。
    let n = view(d, texts, look, width, false).lines.len();
    u16::try_from(n).unwrap_or(u16::MAX).min(max).max(1)
}

/// 按高度裁：放得下全画。放不下时最后两行（空行、按键提示）钉在下面，问题那几行钉在上面，中间的选项跟着
/// 选中那一项滚（照旧版 `panel_layout`）：选项至少留三行（不超过剩下的一半），问题太长留它的后半截。
/// 矮到四行以下（「窗口小的时候」第 3 条）：先去按键提示上面那一行空行，再去问题，最后去按键提示，选项至少留一行；
/// 问题被裁时不留它后面那行空行。交回裁好的样子，顺手把选项滚到哪记进 `scroll`。
pub fn fit(v: View, height: usize, scroll: &mut usize) -> View {
    let n = v.lines.len();
    if n <= height {
        *scroll = 0;
        return v;
    }
    let tail = 2.min(n - v.head);
    let body_len = n - tail - v.head;
    let foot = match height {
        0 | 1 => 0,
        2 | 3 => 1,
        _ => 2,
    }
    .min(tail);
    let avail = height - foot;
    let reserved = body_len
        .min(3)
        .min(avail / 2)
        .max(body_len.min(1))
        .min(avail);
    let top = v.head.min(avail - reserved);
    let room = avail - top;
    // 问题被裁时，留下的是问题的字，不是它后面那行空行。
    let blank = |k: usize| v.lines[k].width() == 0;
    let head_end = if top < v.head {
        (0..v.head).rev().find(|&k| !blank(k)).map_or(0, |k| k + 1)
    } else {
        v.head
    };
    let (first, last) = (v.focus.0 - v.head, v.focus.1 - v.head);
    let mut start = (*scroll).min(body_len.saturating_sub(room));
    if first < start || last - first >= room {
        start = first;
    } else if last >= start + room {
        start = (last + 1).saturating_sub(room);
    }
    *scroll = start;
    // 留下的行在原来的第几行：钉住的几行、问题的后半截、选项的一段、按键提示。
    let pinned = v.pinned.min(top);
    let from = head_end.saturating_sub(top - pinned).max(pinned);
    let kept: Vec<usize> = (0..pinned)
        .chain(from..head_end)
        .chain(v.head + start..(v.head + start + room).min(n - tail))
        .chain(n - foot..n)
        .collect();
    let at = |row: usize| kept.iter().position(|&k| k == row);
    let cursor = v.cursor.and_then(|(row, col)| at(row).map(|r| (r, col)));
    View {
        lines: kept.iter().map(|&k| v.lines[k].clone()).collect(),
        items: kept.iter().map(|&k| v.items[k]).collect(),
        focus: v.focus,
        head: top,
        pinned,
        cursor,
    }
}

/// 画在输入框的字的地方（`area`）；交回每一行是第几项，鼠标照它认。
pub fn draw(
    frame: &mut Frame,
    area: Rect,
    d: &mut Drawer,
    texts: &Texts,
    look: &PreviewLook,
    vim: bool,
    caret: &mut Caret,
) -> Vec<Option<usize>> {
    d.command_rows = d
        .command
        .as_ref()
        .map_or(0, |(_, c)| command::rows(c, area.width).len());
    // 放不下：先少露几行命令（最少 2 行，能滚），空行和问题留着，再照高度裁（2026-10-22 项目主人：「空行也没有」）。
    d.command_window = COMMAND_ROWS;
    // 提问没有命令：同一套滚的是选中那一项的预览（`preview.rs`）。
    if d.command.is_none() {
        (d.command_rows, d.command_window) = preview::scroll(d, texts, look, area.width);
    }
    let mut v = view(d, texts, look, area.width, vim);
    let over = v.lines.len().saturating_sub(usize::from(area.height));
    if over > 0 && d.command.is_some() {
        d.command_window = COMMAND_ROWS.saturating_sub(over).max(2);
        v = view(d, texts, look, area.width, vim);
    }
    let v = fit(v, usize::from(area.height), &mut d.scroll);
    if let Some((row, col)) = v.cursor {
        let x = area.x + col.min(area.width.saturating_sub(1));
        caret.put(
            Position::new(x, area.y + u16::try_from(row).unwrap_or(0)),
            true,
        );
    }
    frame.render_widget(Paragraph::new(v.lines), area);
    v.items
}
