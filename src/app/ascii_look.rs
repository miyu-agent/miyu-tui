//! 纯 ASCII 主题换上、换下时跟着换的（蓝图 `tui.md`「主题」第 6 条，check 13 ①）：界面上的字、命令说明、运行状态行照它重读
//! （`config/localize.rs` 的 `shown`），图标换成 ASCII 那一套，换下来回到选的那一套。

use super::App;
use crate::config::Config;

/// 现在照哪样画、人选的是哪一套图标。
#[derive(Debug, Default)]
pub struct AsciiLook {
    /// 界面上的字是照纯 ASCII 读的。
    on: bool,
    /// 人选的那一套图标（`tui.icons`、`/icons` 换的）；还没选过的是 `None`，照出厂的。
    chosen: Option<String>,
}

impl App {
    /// 换成 `name` 那一套图标（`nerd`、`plain`）；没有这一套的不换。纯 ASCII 主题下记下选的、画 ASCII 那一套。
    pub(super) fn use_icons(&mut self, name: &str) {
        self.media.ascii.chosen = Some(name.to_string());
        let name = if crate::sheet::read(|s| s.ascii) {
            crate::config::ASCII_ICONS
        } else {
            name
        };
        if let Some(set) = self
            .config
            .icon_sets
            .iter()
            .find(|s| s.name == name)
            .cloned()
        {
            self.config.icons = set;
        }
    }

    /// 换了主题以后：只画 ASCII 这一样变了的，界面上的字照它重读、图标跟着换。
    pub(super) fn follow_ascii(&mut self) {
        let ascii = crate::sheet::read(|s| s.ascii);
        if ascii == self.media.ascii.on {
            return;
        }
        self.media.ascii.on = ascii;
        if let Ok(fresh) = Config::load(&self.config.language.clone(), self.config.auto) {
            self.config.text = fresh.text;
            self.config.commands = fresh.commands;
            self.config.pulse = fresh.pulse;
            // 核心认的命令的说明合在命令表里：再问一次
            self.ask_catalog();
        }
        let chosen = self.media.ascii.chosen.clone();
        let chosen = chosen.unwrap_or_else(|| self.config.layout.icons.clone());
        let chosen_now = self.media.ascii.chosen.is_some();
        self.use_icons(&chosen);
        if !chosen_now {
            self.media.ascii.chosen = None;
        }
    }
}
