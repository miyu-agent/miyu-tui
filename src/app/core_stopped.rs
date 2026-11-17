//! 核心是人停的（核心 CS-5「人停的核心，头不再自动拉起」，蓝图 `tui.md`「连核心」第 9 条）：断了线不自己拉起，首页红字写核心
//! 的那一句「核心已停止」，下面一行「Enter 启动」（`ui/above.rs`）。输入框空着按 `Enter`、敲了话、点了要核心的命令都算人要用，
//! 照 `connect_or_start` 拉起（`core/spawn.rs`）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::App;
use crate::transcript::Link;

impl App {
    /// 拉起人停的核心：下一次连照人要用的拉（不等退避），左下角先写「正在连接核心…」。
    pub(super) fn start_core(&mut self) {
        self.link.core.start();
        self.talk.transcript.link = Link::Connecting;
    }

    /// 核心是人停的、输入框空着时按 `Enter`：拉起，交回 `true`。别的照常往下走。
    pub(super) fn stopped_enter(&mut self, key: &KeyEvent) -> bool {
        let stopped = matches!(self.talk.transcript.link, Link::Stopped(_));
        let empty = self.typing.input.editor.text().trim().is_empty();
        if !stopped || !empty || key.code != KeyCode::Enter || !key.modifiers.is_empty() {
            return false;
        }
        self.start_core();
        true
    }
}

#[cfg(test)]
mod tests;
