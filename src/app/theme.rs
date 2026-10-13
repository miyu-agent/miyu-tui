//! 用哪套主题（蓝图 `tui.md`「主题」）：照 `tui.theme*` 的配置项和起来时问到的终端底色挑一套，设好颜色、要不要铺底色；
//! `/theme` 在几种配色之间轮换，只管这一次启动。

use super::App;
use crate::theme::choice::{self, COLORS};
use crate::theme::persona;

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
            self.screen.theme_seed,
        );
        if let Some(picked) = picked {
            crate::theme::set(picked.palette);
            crate::theme::set_ground(picked.ground);
            self.screen.theme = picked.name;
        }
    }

    /// 跟随头像的配色跟着正在看的人格走（「主题」第 5 条）：主色变了重挑一套。每一帧以后看一次（`tick`）。
    pub(super) fn follow_persona(&mut self) {
        if self.screen.theme_choice.colors != "persona" {
            return;
        }
        let seed = self.persona_seed();
        if seed != self.screen.theme_seed {
            self.screen.theme_seed = seed;
            self.apply_theme();
        }
    }

    /// 正在看的人格的主色：人格自己写了的照写的，没写的从头像取（照版本记着，一张只取一次）；都没有、头像还没要到的是
    /// `None`。
    fn persona_seed(&mut self) -> Option<u32> {
        let (seed, version) = {
            let row = self.seen().theme_persona()?;
            (row.seed.clone(), row.avatar.clone())
        };
        if let Some(seed) = seed.as_deref().and_then(persona::hex) {
            return Some(seed);
        }
        let version = version?;
        if let Some(known) = self.screen.seeds.get(&version) {
            return *known;
        }
        let file = self.media.avatars.file(&version)?;
        let seed = persona::seed_of_file(&file);
        self.screen.seeds.insert(version, seed);
        seed
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
