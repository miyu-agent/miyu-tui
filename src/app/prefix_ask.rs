//! 前缀要变时问人（蓝图「前缀要变」，排期第 59 条，2026-11-06 主人定「照网页弹三个按钮」）：开新一轮被拒成
//! `prefix_changing`，先画上的那句留着，输入框上面弹框列出改了的文件，三个按钮「直接发送」「压缩上下文并发送」「取消」。
//! 选了前两个照记着的那一条加 `prefix` 再发（`core/prefix.rs`）；取消照「没发出去」把字放回框里。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Position;
use unicode_width::UnicodeWidthStr;

use super::{App, Panel};
use crate::core::Command;

/// 核心给的：改了的文件、剩下几个、开轮命令的原话。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrefixAsk {
    /// 改了的、真进这个会话前缀的文件（数据根的相对路径）。
    pub files: Vec<String>,
    /// 没列出来的还有几个。
    pub more: u64,
    /// 开轮命令的原话；说的话、重做的是 `None`。
    pub command: Option<String>,
}

/// 直接发送。
pub const KEEP: usize = 0;
/// 压缩上下文并发送。
pub const COMPACT: usize = 1;
/// 取消。
pub const CANCEL: usize = 2;

impl App {
    /// 核心说前缀要变：记下改了的文件，开框，停在「直接发送」。
    pub(super) fn prefix_changing(&mut self, ask: PrefixAsk) {
        self.talk.prefix_ask = Some(ask);
        self.boxes.panel = Some(Panel::Prefix { button: KEEP });
    }

    /// 框开着时的一个键：`←` `→`、`Tab` `Shift+Tab` 挪（到头停），`Enter` 按，`Esc` 取消。
    pub(super) fn prefix_key(&mut self, button: usize, key: KeyEvent) {
        let button = match key.code {
            KeyCode::Left | KeyCode::BackTab => button.saturating_sub(1),
            KeyCode::Right | KeyCode::Tab => (button + 1).min(CANCEL),
            KeyCode::Enter => return self.prefix_press(button),
            KeyCode::Esc => return self.prefix_press(CANCEL),
            _ => button,
        };
        self.boxes.panel = Some(Panel::Prefix { button });
    }

    /// 按下一个按钮：前两个照记着的那一条再发，取消的把字放回框里（开轮命令的不放回，同被拒的斜杠命令）。
    pub(super) fn prefix_press(&mut self, button: usize) {
        self.boxes.panel = None;
        let ask = self.talk.prefix_ask.take();
        match button {
            KEEP => self.link.core.send(Command::Retry("keep")),
            COMPACT => self.link.core.send(Command::Retry("compact")),
            _ if ask.is_some_and(|a| a.command.is_some()) => {}
            _ => self.take_back_unsent(),
        }
    }

    /// 点在框里：点中按钮那一行的哪个按钮就按它（按钮的样子同 `ui/prefix_ask.rs`：`[ 字 ]`，隔两列）。点在框里别处不算点到
    /// 框外面。交回点没点在框上。
    pub(super) fn prefix_click(&mut self, at: Position, press: bool) -> bool {
        let Some(Panel::Prefix { .. }) = self.boxes.panel else {
            return false;
        };
        let areas = self.screen.areas;
        if !areas.menu.contains(at) {
            return false;
        }
        let row = at.y.checked_sub(areas.menu_text.y).map(usize::from);
        let on_buttons = row.and_then(|row| self.screen.panel_rows.get(row).copied().flatten());
        if on_buttons.is_none() || !press {
            return true;
        }
        // 按钮那一行同列表的一行：行首空两格（`ui/panel.rs` 的 `item`）。
        let mut x = areas.menu_text.x + 2;
        for (i, label) in self.config.text.prefix.buttons.iter().enumerate() {
            let w = u16::try_from(format!("[ {label} ]").width()).unwrap_or(0);
            if (x..x + w).contains(&at.x) {
                self.prefix_press(i);
                break;
            }
            x += w + 2;
        }
        true
    }
}
