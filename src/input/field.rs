//! 一处小的写字的地方（2026-11-27 项目主人：抽屉里写「其他」、补充、理由时光标不能挪，换行看不出来，长的只留末尾一截）：
//! 和输入框同一个编辑器（`editor.rs`）、同一套折行（`wrap.rs`）、同一套编辑键（[`edit`]、[`vertical`]、[`line_edge`]）、
//! 同一套大段粘贴收成块（[`paste_into`]）。没有输入框的输入历史、暂存、跑命令的模式这些。
//!
//! 画在哪由外面定：画之前问 [`Field::shown`] 露哪几行、光标在哪，画完照 [`Field::place`] 记下位置，鼠标点一下照它放光标。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Position, Rect};

use super::editor::{self, Editor};
use super::pasted::PasteRule;
use super::wrap::{VisualLine, locate, offset_at, wrap};

/// 输入框和小的写字处都认的编辑键：打字、换行（`Shift+Enter`、`Alt+Enter`、`Ctrl+J`）、左右、按词跳（`Ctrl+←` `Ctrl+→`）、
/// 按词删（`Ctrl+W`、`Ctrl+Backspace`、`Alt+Backspace`）、`Ctrl+A` 回到开头、退格、删除。认了交回 `true`；别的键（`Enter`、
/// 上下、`Home` `End` 这些各处规矩不一样的）交回 `false`，由调的一方处理。
pub(super) fn edit(editor: &mut Editor, key: KeyEvent) -> bool {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    match key.code {
        // Shift+Enter 要终端报得出来（kitty 键盘协议）；报不出来的终端用 Alt+Enter 或 Ctrl+J。
        KeyCode::Enter if shift || alt => editor.insert("\n"),
        KeyCode::Char('j') if ctrl => editor.insert("\n"),
        // 回到开头（2026-09-30 项目主人改：原来是全选）。
        KeyCode::Char('a') if ctrl => editor.move_to(0, false),
        KeyCode::Char('w') if ctrl => editor.delete_word(),
        KeyCode::Backspace if ctrl || alt => editor.delete_word(),
        KeyCode::Left if ctrl => editor.word_left(shift),
        KeyCode::Right if ctrl => editor.word_right(shift),
        KeyCode::Char(c) if !ctrl => editor.insert(c.encode_utf8(&mut [0; 4])),
        KeyCode::Backspace => editor.backspace(),
        KeyCode::Delete => editor.delete(),
        KeyCode::Left => editor.left(shift),
        KeyCode::Right => editor.right(shift),
        _ => return false,
    }
    true
}

/// 在折好的行里往上、往下走 `delta` 行，列照 `goal`（经过短行再到长行回到原来的列）。走得动交回 `true`；第一行再往上、
/// 最后一行再往下交回 `false`，光标不动（输入框这时翻输入历史，抽屉里到开头、结尾）。
pub(super) fn vertical(
    editor: &mut Editor,
    lines: &[VisualLine],
    goal: &mut Option<usize>,
    delta: isize,
    extend: bool,
) -> bool {
    let text = editor.text();
    let (row, col) = locate(text, lines, editor.cursor());
    let goal = *goal.get_or_insert(col);
    let Some(target) = row.checked_add_signed(delta).filter(|&r| r < lines.len()) else {
        return false;
    };
    let pos = offset_at(text, lines, target, goal);
    editor.move_to(pos, extend);
    true
}

/// `Home`、`End`：到光标所在的那一行（折出来的一行）的开头、结尾。
pub(super) fn line_edge(editor: &mut Editor, lines: &[VisualLine], end: bool, extend: bool) {
    let (row, _) = locate(editor.text(), lines, editor.cursor());
    let line = lines[row];
    editor.move_to(if end { line.end } else { line.start }, extend);
}

/// 一共 `total` 行、露 `rows` 行、第一行露的是第 `scroll` 行：把光标所在的第 `row` 行滚进来（`follow` 关着的只管不滚过头）。
pub(super) fn keep_in_view(
    mut scroll: usize,
    (row, rows, total): (usize, usize, usize),
    follow: bool,
) -> usize {
    if follow {
        if row < scroll {
            scroll = row;
        } else if row >= scroll + rows {
            scroll = row + 1 - rows;
        }
    }
    scroll.min(total.saturating_sub(rows))
}

/// 粘贴一段字进光标处：照 `rule` 大段的收成一块（块上写 `[已粘贴 N 行]`，发出去换回原文，蓝图「输入框」第 11 条），
/// 别的照字放进去。
pub(super) fn paste_into(editor: &mut Editor, rule: &PasteRule, text: &str) {
    let clean = editor::clean(text);
    if rule.folds(&clean) {
        let label = rule.label(&clean);
        editor.insert_block(label, clean);
    } else {
        editor.insert(&clean);
    }
}

/// 一处小的写字的地方：字和光标、滚到哪、上下走时想回到的列、上一次画在哪。
#[derive(Debug, Clone, Default)]
pub struct Field {
    editor: Editor,
    /// 第一行露的是折好的第几行。
    scroll: usize,
    /// 上下移动时想回到的列（同输入框）。
    goal_col: Option<usize>,
    /// 上一次画的时候字在屏幕上的位置（宽是折行的宽，高是露几行）；还没画过是空的，上下照原文的换行走。
    area: Rect,
    /// 上一次画的时候，露的头几行又被外面裁掉了几行（抽屉放不下）：屏幕上第一行是折好的第 `scroll + skip` 行。
    skip: usize,
}

/// 这一回露出来的几行：折好的行、第一行是第几行、光标在露出来的第几行第几列。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    /// 露出来的那几行（字节范围照 [`Field::text`]）。
    pub lines: Vec<VisualLine>,
    /// 第一行是折好的第几行。
    pub top: usize,
    /// 光标在露出来的第几行、第几列。
    pub cursor: (usize, usize),
}

impl Field {
    /// 写字处里看得到的字：粘贴块写的是块上的字。
    pub fn text(&self) -> &str {
        self.editor.text()
    }

    /// 同 [`Field::text`]（测试里照字符串比）。
    #[cfg(test)]
    pub fn as_str(&self) -> &str {
        self.editor.text()
    }

    /// 编辑器：画的时候要选区、粘贴块的位置。
    pub fn editor(&self) -> &Editor {
        &self.editor
    }

    /// 只有空白，或者什么都没写。
    pub fn is_blank(&self) -> bool {
        self.editor.text().trim().is_empty()
    }

    /// 交出去的全文：粘贴块换回原文。
    pub fn full(&self) -> String {
        self.editor.draft().expand()
    }

    /// 清空。
    pub fn clear(&mut self) {
        self.editor.take();
        self.scroll = 0;
        self.goal_col = None;
    }

    /// 处理一个编辑键：[`edit`] 认的照输入框；上下在折好的行里走，第一行再往上到开头、最后一行再往下到结尾（按着 `Shift`
    /// 的选到那里）；`Home` `End` 到这一行的开头、结尾。认了交回 `true`。`Enter`、`Esc` 不归这里。
    pub fn key(&mut self, key: KeyEvent) -> bool {
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        if !matches!(key.code, KeyCode::Up | KeyCode::Down) {
            self.goal_col = None;
        }
        match key.code {
            KeyCode::Up | KeyCode::Down => {
                let delta = if key.code == KeyCode::Up { -1 } else { 1 };
                let lines = self.lines(self.width());
                if !vertical(&mut self.editor, &lines, &mut self.goal_col, delta, shift) {
                    let edge = if delta < 0 {
                        0
                    } else {
                        self.editor.text().len()
                    };
                    self.editor.move_to(edge, shift);
                }
                true
            }
            KeyCode::Home | KeyCode::End => {
                let lines = self.lines(self.width());
                line_edge(&mut self.editor, &lines, key.code == KeyCode::End, shift);
                true
            }
            _ => edit(&mut self.editor, key),
        }
    }

    /// 粘贴一段字进光标处，大段的照 `rule` 收成一块（同输入框）。
    pub fn paste(&mut self, text: &str, rule: &PasteRule) {
        self.goal_col = None;
        paste_into(&mut self.editor, rule, text);
    }

    /// 照 `width` 列折好的行（空的也有一行）。
    pub fn lines(&self, width: u16) -> Vec<VisualLine> {
        wrap(self.editor.text(), width)
    }

    /// 照 `width` 列折、最多露 `cap` 行：露哪几行（光标那一行一定露着，照上一回滚到的地方少挪），光标在哪。
    pub fn shown(&self, width: u16, cap: usize) -> Shown {
        let lines = self.lines(width);
        let text = self.editor.text();
        let (row, col) = locate(text, &lines, self.editor.cursor());
        let rows = cap.clamp(1, lines.len().max(1));
        let top = keep_in_view(self.scroll, (row, rows, lines.len()), true);
        Shown {
            lines: lines[top..(top + rows).min(lines.len())].to_vec(),
            top,
            cursor: (row - top, col),
        }
    }

    /// 画完记下：字画在屏幕的 `area`（宽是折行的宽、高是屏幕上露几行），[`Field::shown`] 露的第一行是第 `top` 行，外面又裁掉了
    /// 头 `skip` 行。
    pub fn place(&mut self, area: Rect, top: usize, skip: usize) {
        self.area = area;
        self.scroll = top;
        self.skip = skip;
    }

    /// 鼠标点在 `at`：落在字上的，光标挪过去，交回 `true`（照输入框：点在宽字的右半边落在它后面，点过行尾落在行尾）。
    pub fn click(&mut self, at: Position) -> bool {
        let a = self.area;
        if !a.contains(at) {
            return false;
        }
        let lines = self.lines(a.width);
        let row = self.scroll + self.skip + usize::from(at.y - a.y);
        let pos = offset_at(self.editor.text(), &lines, row, usize::from(at.x - a.x));
        self.goal_col = None;
        self.editor.move_to(pos, false);
        true
    }

    /// 上下走照的宽：画过的照画的宽，还没画过的不折（只照原文的换行）。
    fn width(&self) -> u16 {
        if self.area.width == 0 {
            u16::MAX
        } else {
            self.area.width
        }
    }
}

impl From<&str> for Field {
    /// 写好一段字，光标在末尾（照 [`Editor::set`] 洗过）。
    fn from(text: &str) -> Self {
        let mut field = Self::default();
        field.editor.set(text);
        field
    }
}

impl PartialEq<&str> for Field {
    fn eq(&self, other: &&str) -> bool {
        self.editor.text() == *other
    }
}

impl PartialEq<str> for Field {
    fn eq(&self, other: &str) -> bool {
        self.editor.text() == other
    }
}
