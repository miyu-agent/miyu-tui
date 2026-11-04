//! 跑命令的模式（蓝图 `tui.md`「人自己跑命令 `!`」第 1、5 条，排期第 21 条）：框里头一个字是英文 `!` 的进这个模式，`!` 不留在
//! 框里；空着按 `Backspace`、`Esc` 回到平常。打的、粘进来的、`↑` 翻出来的、退回来的都照这一条认（输入历史里记成 `!` 打头的一条）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::{Draft, InputBox};

/// 跑命令的那一条记进输入历史、放回框里时打头的字。
pub const BANG: char = '!';

impl InputBox {
    /// 在跑命令的模式里。
    pub fn shell(&self) -> bool {
        self.shell
    }

    /// 回到平常的输入（发出去了、清空了）。
    pub fn leave_shell(&mut self) {
        self.shell = false;
    }

    /// 空着按 `Backspace`、`Esc`：回到平常的输入，交回 `true`（这个键用掉了）。编辑上一句时 `Esc` 照旧是取消。
    pub(super) fn shell_key(&mut self, key: KeyEvent) -> bool {
        let leave =
            matches!(key.code, KeyCode::Backspace) || (key.code == KeyCode::Esc && !self.editing());
        if self.shell && self.editor.is_empty() && leave {
            self.shell = false;
            return true;
        }
        false
    }

    /// 框里头一个字是 `!`：去掉它，进跑命令的模式。
    pub fn absorb_bang(&mut self) {
        if self.editor.text().starts_with(BANG) {
            let draft = shift(self.editor.draft(), false);
            self.editor.set_draft(draft);
            self.editor.move_to(0, false);
            self.shell = true;
        }
    }

    /// 放进框里一句（翻历史、挑历史、退回来的）：`!` 打头的进跑命令的模式，别的回到平常。
    pub(super) fn load(&mut self, draft: Draft) {
        self.shell = false;
        self.editor.set_draft(draft);
        self.absorb_bang();
        if self.shell {
            self.editor.move_to(self.editor.text().len(), false);
        }
    }

    /// 框里这一句记进输入历史、放回框里时的样子：跑命令的前面加 `!`。
    pub fn as_typed(&self, draft: &Draft) -> Draft {
        if self.shell {
            shift(draft.clone(), true)
        } else {
            draft.clone()
        }
    }
}

/// 打头加上（`add`）、去掉一个 `!`，粘贴块的位置跟着挪。
fn shift(mut draft: Draft, add: bool) -> Draft {
    let n = BANG.len_utf8();
    if add {
        draft.text.insert(0, BANG);
    } else {
        draft.text.remove(0);
    }
    for block in &mut draft.blocks {
        if add {
            block.start += n;
            block.end += n;
        } else {
            block.start = block.start.saturating_sub(n);
            block.end = block.end.saturating_sub(n);
        }
    }
    draft
}

#[cfg(test)]
mod tests;
