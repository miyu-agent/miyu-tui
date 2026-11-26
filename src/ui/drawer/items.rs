//! 抽屉里的选项、补充那一行、「确认」页（蓝图「确认和提问的抽屉」第 3 条）。

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::View;
use super::field::{self, Spot, indented, settled};
use crate::drawer::{Decision, Drawer, Edit, Item, Texts, said};
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
    /// 编辑时写字的那一块在哪。
    pub field: Option<Spot>,
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
        if let Some(spot) = self.field {
            v.field = Some(Spot {
                row: at + spot.row,
                ..spot
            });
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
        Item::Other => !typed.is_blank(),
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
    // 序号：按 1–9 直接选这一项（`drawer/keys.rs`；2026-10-27 项目主人：选项加上序号，照 Claude Code），一律暗色
    // （2026-11-02 项目主人）。
    let number = format!("{}. ", index + 1);
    let mut spans = vec![
        Span::styled(pointer.clone(), theme::picked()),
        Span::styled(number.clone(), theme::dim()),
    ];
    if let Some(marker) = marker {
        spans.push(Span::styled(marker.clone(), style));
    }
    let lead = pointer.width() + number.width() + marker.map_or(0, |m| m.width());
    let room = usize::from(width).saturating_sub(lead);
    // 在写的那一项：「其他」下面一块写字的地方，和标题对齐；理由接在标题后面空两格。和输入框同一个编辑器（2026-11-27）。
    let editing = active
        && matches!(
            (d.editing, item),
            (Some(Edit::Other), Item::Other) | (Some(Edit::Reason), Item::Decision(Decision::Deny))
        );
    let (label, description) = match item {
        Item::Choice(i) => {
            let option = d.question_at(tab).map(|q| &q.options[i]);
            // 模型写的字：控制字符（制表符、ESC 这类）不进格子，不然终端多走几列、把并排的竖线顶歪（2026-11-11 项目主人）。
            let clean = |text: &String| crate::tabs::printable(text).into_owned();
            (
                option.map_or(String::new(), |o| clean(&o.label)),
                option.and_then(|o| o.description.as_ref().map(clean)),
            )
        }
        // 写好了的照写的字，不加「自定义：」（2026-10-20 项目主人：「编辑完成后不需要『自定义』」）；在写的照旧写「输入其他答案」，
        // 字写在下面。
        Item::Other if !editing && !typed.is_blank() => (typed.text().to_string(), None),
        Item::Other => (texts.other.clone(), None),
        Item::Decision(decision) => {
            let at = Decision::ALL
                .iter()
                .position(|x| *x == decision)
                .unwrap_or(0);
            (texts.decisions[at].clone(), None)
        }
    };
    // 放不下的标题折行、不截（2026-11-02 项目主人）；在写理由的「不允许」后面接着写字，照旧截。写好的「其他」照原文的换行
    // 分行、折行，最多几行（`field.rs` 的 `settled`；原来换行写成 `↵` 挤在一行里）。
    let mut labels = match item {
        Item::Other if !editing && !typed.is_blank() => settled(&label, col(room)),
        Item::Decision(_) if editing => vec![super::clip(&label, col(room))],
        _ => folded(&label, col(room)),
    }
    .into_iter();
    let label = labels.next().unwrap_or_default();
    let mut used = lead + label.width();
    spans.push(Span::styled(label, label_style));
    let indent = " ".repeat(lead);
    let mut more: Vec<Line<'static>> = labels
        .map(|row| {
            Line::from(vec![
                Span::raw(indent.clone()),
                Span::styled(row, label_style),
            ])
        })
        .collect();
    // 「其他」写字的那几行：接在标题下面，和标题对齐（2026-11-27，照 LiteMiyu）。
    let mut below = Vec::new();
    let cap = d.field_rows.get();
    if editing && item == Item::Other {
        let room = col(usize::from(width).saturating_sub(lead + 1)).max(1);
        let rows = field::rows(typed, room, cap, &texts.other);
        let first = b.lines.len() + 1 + more.len();
        b.cursor = Some((first + rows.cursor.0, col(lead) + rows.cursor.1));
        b.field = Some(Spot {
            row: first,
            col: col(lead),
            width: room,
            rows: rows.lines.len(),
            top: rows.top,
            skip: 0,
        });
        below.extend(rows.lines.into_iter().map(|r| indented(lead, r)));
    } else if editing {
        // 理由能分行（2026-10-22 项目主人：「理由不支持换行」）：第一行接在「不允许」后面空两格，换了行的、写长了的往下
        // 对齐接着写。
        spans.push(Span::raw("  "));
        used += 2;
        let room = col(usize::from(width).saturating_sub(used + 1)).max(1);
        let rows = field::rows(typed, room, cap, &texts.reason_hint);
        let first = b.lines.len();
        b.cursor = Some((first + rows.cursor.0, col(used) + rows.cursor.1));
        b.field = Some(Spot {
            row: first,
            col: col(used),
            width: room,
            rows: rows.lines.len(),
            top: rows.top,
            skip: 0,
        });
        let mut rows = rows.lines.into_iter();
        spans.extend(rows.next().unwrap_or_default());
        more.extend(rows.map(|r| indented(used, r)));
    }
    // 光标停在「不允许」上、还没在写：后面暗色写怎么写理由（2026-10-22 项目主人：「鼠标移动到不允许，然后出现 n 键
    // 输入理由」）。
    if active && !editing && item == Item::Decision(Decision::Deny) && d.editing.is_none() {
        spans.push(Span::styled(
            format!("  {}", texts.reason_key),
            theme::dim(),
        ));
    }
    b.push(Line::from(spans), Some(index));
    for line in more.into_iter().chain(below) {
        b.push(line, Some(index));
    }
    if let Some(description) = description.filter(|s| !s.trim().is_empty()) {
        let room = col(usize::from(width).saturating_sub(lead)).max(1);
        for piece in folded(description.trim(), room) {
            let spans = vec![Span::raw(indent.clone()), Span::styled(piece, theme::dim())];
            b.push(Line::from(spans), Some(index));
        }
    }
}

/// 按 `n` 写的补充：有字或者正在写时，选项下面 `补充  字`；写长了的、换了行的往下和第一行的字对齐（2026-11-27，同
/// 「其他」）。
pub fn notes(d: &Drawer, texts: &Texts, width: u16, v: &mut View) {
    let text = &d.notes[d.tab];
    let editing = d.editing == Some(Edit::Notes);
    if text.is_blank() && !editing {
        return;
    }
    let lead = format!(
        "{}{}  ",
        " ".repeat(texts.pointer.width()),
        texts.notes_label
    );
    let lw = lead.width();
    let room = col(usize::from(width).saturating_sub(lw + 1)).max(1);
    let rows: Vec<Vec<Span<'static>>> = if editing {
        // 补充没有现成的提示语：空着只写「补充」。
        let rows = field::rows(text, room, d.field_rows.get(), "");
        v.cursor = Some((v.lines.len() + rows.cursor.0, col(lw) + rows.cursor.1));
        v.field = Some(Spot {
            row: v.lines.len(),
            col: col(lw),
            width: room,
            rows: rows.lines.len(),
            top: rows.top,
            skip: 0,
        });
        rows.lines
    } else {
        settled(text.text(), room)
            .into_iter()
            .map(|r| vec![Span::raw(r)])
            .collect()
    };
    let mut rows = rows.into_iter();
    let mut first = vec![Span::styled(lead, theme::dim())];
    first.extend(rows.next().unwrap_or_default());
    v.push(Line::from(first), None);
    for row in rows {
        v.push(indented(lw, row), None);
    }
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

/// 照 `room` 列折好的几行（至少一行），每一行去掉行尾的空白：正文的折行把满了的那一格空白留在行尾，多占一列，
/// 抽屉里并排时中线会错开（2026-11-02 项目主人报）。
pub(super) fn folded(text: &str, room: u16) -> Vec<String> {
    let mut out: Vec<String> = pieces(text, room)
        .into_iter()
        .map(|(piece, _)| piece.trim_end().to_string())
        .collect();
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn col(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}
