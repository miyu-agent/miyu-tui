//! 用哪套主题（蓝图 `tui.md`「主题」）：照 `tui.theme*` 的配置项和起来时问到的终端底色挑一套，设好颜色、要不要铺底色；
//! `/theme` 在几种配色之间轮换，只管这一次启动。

use super::App;
use crate::theme::choice::{self, COLORS};

impl App {
    /// 起来时问到了终端的底色：照它重挑（跟随终端的才看它）。
    pub fn set_terminal_dark(&mut self, dark: Option<bool>) {
        self.screen.terminal_dark = dark;
        self.apply_theme();
    }

    /// 照现在的配置项、终端的底色挑一套，设上。
    pub(super) fn apply_theme(&mut self) {
        let picked = choice::pick(
            &self.screen.theme_choice,
            self.screen.terminal_dark,
            &self.config.themes,
        );
        if let Some(picked) = picked {
            crate::theme::set(picked.palette);
            crate::theme::set_ground(picked.ground);
            self.screen.theme = picked.name;
        }
    }

    /// 换下一种配色，只管这一次启动（「主题」第 6 条），提示「主题：名字」。
    pub(super) fn next_theme(&mut self) {
        let colors = &mut self.screen.theme_choice.colors;
        let at = COLORS.iter().position(|c| c == colors).unwrap_or(0);
        *colors = COLORS[(at + 1) % COLORS.len()].to_string();
        self.apply_theme();
        let text = self
            .config
            .text
            .theme_changed
            .replace("{name}", &self.screen.theme);
        self.hint(text, true);
    }
}
