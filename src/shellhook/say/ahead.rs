//! 她答话时你敲的字（排期第 46 条，2026-11-04 项目主人：「行为应该和全屏完全相同才对的」）：用全屏输入框那一套（`input::InputBox`），
//! 左右挪、Home/End、Shift+Enter 换行、删词、括号粘贴、Ctrl+V 读剪贴板（图、文件收成块）都同全屏。回车插话时块换回原文、图跟着发；
//! 她答完了没发出去的交还给 shell 的命令行，块照 `shellhook paste` 记进这个 shell 的那一份（同在 shell 里按 Ctrl+V），说话时换回来。

use ratatui::crossterm::event::KeyEvent;
use ratatui::text::Line;

use crate::config::Config;
use crate::input::{Draft, InputBox, VisualLine, locate};

/// 提前敲的字。
pub struct Ahead {
    input: InputBox,
    /// 哪个 shell 说的（`--shell`）：块记进它的那一份、截图暂存在它名下；没挂钩子的照这个进程。
    shell: Option<u32>,
    /// 收块的规矩已经照配置设好了。
    ready: bool,
}

impl Default for Ahead {
    fn default() -> Self {
        Self {
            input: InputBox::new(8, std::time::Duration::from_millis(400)),
            shell: None,
            ready: false,
        }
    }
}

impl Ahead {
    /// 这个 shell 说的。
    pub fn set_shell(&mut self, shell: Option<u32>) {
        self.shell = shell;
        self.ready = false;
    }

    /// 什么都没敲。
    pub fn is_empty(&self) -> bool {
        self.input.editor.is_empty()
    }

    fn owner(&self) -> u32 {
        self.shell.unwrap_or_else(std::process::id)
    }

    /// 头一回用以前照配置设好收块的规矩（大段的字折成块、附件的写法，同全屏）。
    fn prepare(&mut self, config: &Config) {
        if !self.ready {
            let owner = self.owner();
            crate::shellhook::paste::rules(&mut self.input, config, owner);
            self.ready = true;
        }
    }

    /// 一个键照全屏输入框办（回车、Ctrl+C 外面先拿走）。
    pub fn key(&mut self, key: KeyEvent, config: &Config) {
        self.prepare(config);
        drop(self.input.key(key));
    }

    /// 括号粘贴：照全屏，长的收成块、本机的文件收成附件块。
    pub fn paste(&mut self, text: &str, config: &Config) {
        self.prepare(config);
        if !self.input.paste_files(text) {
            self.input.paste(text);
        }
    }

    /// Ctrl+V：照全屏读剪贴板（复制的文件、截图、字）。读不到的不动。
    pub fn clipboard(&mut self, config: &Config) {
        self.prepare(config);
        let owner = self.owner();
        // 读不到、是空的不动（全屏弹一句提示；shell 里活动区没有提示的地方）。
        crate::shellhook::paste::grab(&mut self.input, owner, config).unwrap_or_default();
    }

    /// 框里的话（块换回原文）放得进协议的一行（1 MiB，施工 YC-11）。
    pub fn fits(&self) -> bool {
        crate::core::line_limit::fits(&self.input.draft().expand())
    }

    /// 回车插话：交出框里的（块还是块，[`Draft::expand`] 换回原文、[`Draft::attachments`] 是图），框清空。
    pub fn take(&mut self) -> Draft {
        self.input.editor.take_draft()
    }

    /// 插的话被退回来（Ctrl+C）：一条一行放回框里，接在已经敲的前面。
    pub fn put_back(&mut self, back: Vec<String>) {
        let mut draft = Draft::default();
        for text in back {
            draft.append(Draft::plain(&text), "\n");
        }
        if !self.is_empty() {
            draft.append(self.input.draft(), "\n");
        }
        self.input.editor.set_draft(draft);
    }

    /// 交还给 shell 的命令行：块记进这个 shell 的那一份（[`crate::shellhook::paste::remember`]），交回命令行上写的字。
    pub fn hand_back(&self) -> String {
        let draft = self.input.draft();
        if let Some(shell) = self.shell {
            drop(crate::shellhook::paste::remember(shell, &draft));
        }
        draft.text
    }

    /// 排成的几行（`room` 列宽、最多 `rows` 行，光标那一行露着），和光标在第几行（从上往下数）、第几列。块照全屏输入框的样子
    /// 写（品红、选中的反色）。
    pub fn rows(&self, room: u16, rows: usize) -> (Vec<Line<'static>>, (usize, usize)) {
        let editor = &self.input.editor;
        let text = editor.text();
        let lines: Vec<VisualLine> = self.input.lines(room.max(1));
        let (row, col) = locate(text, &lines, editor.cursor());
        let rows = rows.max(1);
        let top = (row + 1).saturating_sub(rows);
        let blocks = editor.blocks();
        let shown = lines
            .iter()
            .skip(top)
            .take(rows)
            .map(|l| {
                let line = crate::ui::styled_line(text, *l, editor.selection(), &blocks, None);
                Line::from(
                    line.spans
                        .into_iter()
                        .map(|s| ratatui::text::Span::styled(s.content.into_owned(), s.style))
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        (shown, (row - top, col))
    }
}

#[cfg(test)]
mod tests;
