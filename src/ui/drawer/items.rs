//! 抽屉里的选项、补充那一行、「确认」页（蓝图「确认和提问的抽屉」第 3 条）。

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::View;
use crate::drawer::{Ask, Decision, Drawer, Edit, Item, Texts, inline, said};
use crate::input::pieces;
use crate::theme;

/// 一块排好的行，行号从这一块的第一行数：选项那一块要和右边的文字画并排，先单独排。
#[derive(Debug, Default)]
pub struct Block {
    /// 每一行。
    pub lines: Vec<Line<'static>>,
    /// 每一行是第几项。
    pub items: Vec<Option<usize>>,
    /// 选中那一项的第一行、最后一行。
    pub focus: (usize, usize),
    /// 编辑时光标的行、列。
    pub cursor: Option<(usize, u16)>,
}

impl Block {
    fn push(&mut self, line: Line<'static>, item: Option<usize>) {
        self.lines.push(line);
        self.items.push(item);
    }

    /// 接到 `v` 后面，行号照接上的位置挪。
    pub fn append_to(self, v: &mut View) {
        let at = v.lines.len();
        v.focus = (at + self.focus.0, at + self.focus.1);
        if let Some((row, col)) = self.cursor {
            v.cursor = Some((at + row, col));
        }
        v.lines.extend(self.lines);
        v.items.extend(self.items);
    }
}

/// 选项：一项一行标题（加粗；选中的行首 `› `、品红；能多选的前面 `[ ]`），下面暗色说明。
pub fn choices(d: &Drawer, texts: &Texts, width: u16) -> Block {
    let mut b = Block::default();
    for (index, item) in d.items(d.tab).into_iter().enumerate() {
        let first = b.lines.len();
        item_lines(d, texts, width, index, item, &mut b);
        if index == d.cursor[d.tab] {
            b.focus = (first, b.lines.len() - 1);
        }
    }
    images(d, texts, width, &mut b);
    b
}

/// 这一页贴的图（核心 Q-A，「确认和提问的抽屉」第 3a 条）：选项下面一行小块，同输入框的块；放不下的折到下一行。
fn images(d: &Drawer, texts: &Texts, width: u16, b: &mut Block) {
    let Some(images) = d.images.get(d.tab).filter(|i| !i.is_empty()) else {
        return;
    };
    let indent = " ".repeat(texts.pointer.width());
    let room = usize::from(width);
    let mut spans = vec![Span::raw(indent.clone())];
    let mut used = indent.width();
    for pasted in images {
        let w = pasted.label.width();
        if used > indent.width() && used + 1 + w > room {
            b.push(Line::from(std::mem::take(&mut spans)), None);
            spans.push(Span::raw(indent.clone()));
            used = indent.width();
        }
        if used > indent.width() {
            spans.push(Span::raw(" "));
            used += 1;
        }
        spans.push(Span::styled(pasted.label.clone(), theme::chip()));
        used += w;
    }
    b.push(Line::from(spans), None);
}

fn item_lines(d: &Drawer, texts: &Texts, width: u16, index: usize, item: Item, b: &mut Block) {
    let tab = d.tab;
    let active = index == d.cursor[tab];
    let typed = &d.typed[tab];
    let pointer = if active {
        texts.pointer.clone()
    } else {
        " ".repeat(texts.pointer.width())
    };
    let checked = match item {
        Item::Choice(i) => d.checked[tab].get(i) == Some(&true),
        Item::Other => !typed.trim().is_empty(),
        Item::Decision(_) => false,
    };
    let mark = if checked {
        &texts.checked
    } else {
        &texts.unchecked
    };
    let marker = (d.multiple(tab) && matches!(item, Item::Choice(_) | Item::Other)).then_some(mark);
    let lit = active || (checked && d.multiple(tab));
    let style = if lit { theme::picked() } else { Style::new() };
    let label_style = style.add_modifier(Modifier::BOLD);
    // 序号：按 1–9 直接选这一项（`drawer/keys.rs`；2026-10-27 项目主人：选项加上序号，照 Claude Code）。
    let number = format!("{}. ", index + 1);
    let mut spans = vec![
        Span::styled(pointer.clone(), theme::picked()),
        Span::styled(number.clone(), style),
    ];
    if let Some(marker) = marker {
        spans.push(Span::styled(marker.clone(), style));
    }
    let lead = pointer.width() + number.width() + marker.map_or(0, |m| m.width());
    let room = usize::from(width).saturating_sub(lead);
    // 在写的那一项：标题换成写的字（「其他」），或者标题后面空两格接写的字（理由）；光标跟在字后面。
    let editing = active
        && matches!(
            (d.editing, item),
            (Some(Edit::Other), Item::Other) | (Some(Edit::Reason), Item::Decision(Decision::Deny))
        );
    let (label, description) = match item {
        Item::Choice(i) => {
            let option = d.question_at(tab).map(|q| &q.options[i]);
            (
                option.map_or(String::new(), |o| o.label.clone()),
                option.and_then(|o| o.description.clone()),
            )
        }
        Item::Other if editing => (String::new(), None),
        // 写好了的照写的字，不加「自定义：」（2026-10-20 项目主人：「编辑完成后不需要『自定义』」）。
        Item::Other if !typed.trim().is_empty() => (inline(typed), None),
        Item::Other => (texts.other.clone(), None),
        Item::Decision(decision) => {
            let at = Decision::ALL
                .iter()
                .position(|x| *x == decision)
                .unwrap_or(0);
            // 规则带命令前缀的写清是哪种命令（2026-10-27 项目主人照推荐定：「按命令前缀记住」）。
            let prefix = match &d.ask {
                Ask::Approval(a) if decision == Decision::Session => a.prefix(),
                _ => None,
            };
            let label = prefix.map_or_else(
                || texts.decisions[at].clone(),
                |p| texts.decision_prefix.replace("{prefix}", p),
            );
            (label, None)
        }
    };
    let label = super::clip(&label, u16::try_from(room).unwrap_or(u16::MAX));
    let mut used = lead + label.width();
    spans.push(Span::styled(label, label_style));
    let mut more = Vec::new();
    if editing {
        let (gap, hint) = match item {
            Item::Other => ("", &texts.other),
            _ => ("  ", &texts.reason_hint),
        };
        spans.push(Span::raw(gap));
        used += gap.width();
        let room = usize::from(width).saturating_sub(used + 1);
        if item == Item::Other || typed.is_empty() {
            let shown = tail(&inline_raw(typed), room);
            b.cursor = Some((b.lines.len(), col(used + shown.width())));
            if shown.is_empty() {
                spans.push(Span::styled(tail(hint, room), theme::dim()));
            } else {
                spans.push(Span::raw(shown));
            }
        } else {
            // 理由能分行（2026-10-22 项目主人：「理由不支持换行」）：第一行接在「不允许」后面，换了行的、写长了的往下对齐
            // 接着写，光标跟在最后。
            let rows = reason_rows(typed, room);
            let last = rows.last().map_or(0, |r| r.width());
            b.cursor = Some((b.lines.len() + rows.len() - 1, col(used + last)));
            let mut rows = rows.into_iter();
            spans.push(Span::raw(rows.next().unwrap_or_default()));
            let indent = " ".repeat(used);
            more.extend(rows.map(|r| Line::from(vec![Span::raw(indent.clone()), Span::raw(r)])));
        }
    }
    b.push(Line::from(spans), Some(index));
    for line in more {
        b.push(line, Some(index));
    }
    if let Some(description) = description.filter(|s| !s.trim().is_empty()) {
        let indent = " ".repeat(lead);
        let room = col(usize::from(width).saturating_sub(lead)).max(1);
        for (piece, _) in pieces(description.trim(), room) {
            let spans = vec![Span::raw(indent.clone()), Span::styled(piece, theme::dim())];
            b.push(Line::from(spans), Some(index));
        }
    }
}

/// 按 `n` 写的补充：有字或者正在写时，选项下面一行 `补充  字`。
pub fn notes(d: &Drawer, texts: &Texts, width: u16, v: &mut View) {
    let text = &d.notes[d.tab];
    let editing = d.editing == Some(Edit::Notes);
    if text.trim().is_empty() && !editing {
        return;
    }
    let lead = format!(
        "{}{}  ",
        " ".repeat(texts.pointer.width()),
        texts.notes_label
    );
    let room = usize::from(width).saturating_sub(lead.width() + 1);
    let shown = tail(&inline_raw(text), room);
    if editing {
        v.cursor = Some((v.lines.len(), col(lead.width() + shown.width())));
    }
    v.push(
        Line::from(vec![Span::styled(lead, theme::dim()), Span::raw(shown)]),
        None,
    );
}

/// 「确认」页：一道一行 `短名：回答`，短名加粗、回答暗，没答的红色「未回答」。
pub fn review(d: &Drawer, texts: &Texts, width: u16, v: &mut View) {
    let bold = Style::new().add_modifier(Modifier::BOLD);
    for page in 0..d.pages() {
        let label = format!("{}：", d.label(page));
        let used = label.width();
        let room = col(usize::from(width).saturating_sub(used));
        let (answer, style) = match d.answers[page].as_ref() {
            Some(a) => (said(a), theme::dim()),
            None => (texts.unanswered.clone(), theme::error()),
        };
        v.push(
            Line::from(vec![
                Span::styled(label, bold),
                Span::styled(super::clip(&answer, room), style),
            ]),
            None,
        );
    }
}

/// 写的字排成一行：换行写成 `↵`，前后的空白照留（光标跟着）。
/// 理由排成的几行：照换行分开，长的照 `room` 折行；空行照样占一行。
fn reason_rows(text: &str, room: usize) -> Vec<String> {
    let room = u16::try_from(room).unwrap_or(u16::MAX).max(1);
    let mut out = Vec::new();
    for line in text.split('\n') {
        let parts = pieces(line, room);
        if parts.is_empty() {
            out.push(String::new());
        }
        out.extend(parts.into_iter().map(|(piece, _)| piece));
    }
    out
}

fn inline_raw(text: &str) -> String {
    text.replace(['\r', '\n'], &crate::sheet::mark(|m| &m.enter))
}

/// 放进 `width` 列：放不下的留后面那截（看得到最新写的）。
fn tail(text: &str, width: usize) -> String {
    crate::cells::tail(text, width)
}

fn col(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}
