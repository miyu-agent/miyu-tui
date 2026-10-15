//! 用哪套主题（蓝图 `tui.md`「主题」）：照 `tui.theme*` 的配置项（引导「主题」那一步选着的盖在上面）、起来时问到的终端
//! 底色、跟随头像的主色挑一套，设好颜色、要不要铺底色；`/theme` 在几种配色之间轮换，只管这一次启动。

use super::App;
use crate::oobe::Oobe;
use crate::oobe::persona::Colors;
use crate::theme::choice::{self, Choice};
use crate::theme::persona;

impl App {
    /// 起来时问到了终端的底色：照它重挑（跟随终端的才看它）。
    pub fn set_terminal_dark(&mut self, dark: Option<bool>) {
        self.screen.terminal_dark = dark;
        self.apply_theme();
    }

    /// 现在照哪几样挑：配置项，引导「主题」那一步选着的盖在上面（「第一次打开的引导」第 23a 条）。
    fn choice(&self) -> Choice {
        let mut choice = self.screen.theme_choice.clone();
        if let Some((colors, mode)) = &self.screen.theme_preview {
            choice.colors.clone_from(colors);
            choice.mode = *mode;
        }
        choice
    }

    /// 照现在的配置项、终端的底色、跟随头像的主色挑一套，设上。
    pub(super) fn apply_theme(&mut self) {
        let picked = choice::pick(
            &self.choice(),
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

    /// 每一帧以后看一次（`tick`）：引导「主题」那一步选着的变了、跟随头像的主色变了（换了会话、人格框挪了、头像要到了、
    /// 取好了），重挑一套（「主题」第 5 条）。
    pub(super) fn follow_theme(&mut self) {
        self.follow_wallpaper();
        let preview = self.boxes.oobe.as_ref().and_then(Oobe::theme_preview);
        let changed = preview != self.screen.theme_preview;
        self.screen.theme_preview = preview;
        let seed = self.wanted_seed();
        if changed || seed != self.screen.theme_seed {
            self.screen.theme_seed = seed;
            self.apply_theme();
        }
        self.share_seeds();
    }

    /// 跟随头像的配色照谁取色：引导开着的照人格那一步那一位，不然照正在看的人格（`Seen::theme_persona`）。
    pub(super) fn colors_source(&self) -> Option<Colors> {
        if let Some(oobe) = &self.boxes.oobe {
            return oobe.persona.colors();
        }
        let row = self.seen().theme_persona()?;
        let seeds = row.seeds.clone().unwrap_or_default();
        (row.seed.is_some() || !seeds.is_empty()).then(|| Colors::Given {
            seed: row.seed.clone(),
            seeds,
        })
    }

    /// 跟随壁纸（「主题」的「跟随壁纸」）：起来时找一次，打开配色列表、选上跟随壁纸时再找一次，不定时查（2026-10-14 项目主人：
    /// 不要轮询）；找没找到交给两处配色列表（找不到的不列）。跟随着才解图取色、才盯文件（第 6 条）。
    fn follow_wallpaper(&mut self) {
        let following = self.choice().colors == "wallpaper";
        let wanted = self.colors_list_open() || following;
        let dark = self.choice().dark(self.screen.terminal_dark);
        let wallpaper = &mut self.screen.wallpaper;
        // 盯着的文件变了（换了壁纸）：再找一次（不定时查，2026-10-14 项目主人）。
        let changed = wallpaper.take_changed();
        if !self.screen.wallpaper_seen || (wanted && !self.screen.wallpaper_wanted) || changed {
            self.screen.wallpaper_seen = true;
            wallpaper.look(dark, following);
        } else if following && wallpaper.undecoded() {
            // 刚选上跟随壁纸：找到的图还没解，解开取色。
            wallpaper.look(dark, true);
        }
        wallpaper.follow(following);
        self.screen.wallpaper_wanted = wanted;
        // 找好了、变了：记下候选主色，下次起来先照它（`look.rs`）。
        if self.screen.wallpaper.collect() {
            self.note_wallpaper();
        }
        let available = self.screen.wallpaper.available();
        if let Some(oobe) = &mut self.boxes.oobe {
            oobe.theme.wallpaper = available;
        }
        if let Some(settings) = &mut self.boxes.settings {
            settings.more.wallpaper = available;
        }
    }

    /// 窗口拿回焦点：选着跟随壁纸的再找一次。
    pub(super) fn wallpaper_focus(&mut self) {
        if self.choice().colors == "wallpaper" {
            let dark = self.choice().dark(self.screen.terminal_dark);
            self.screen.wallpaper.look(dark, true);
        }
    }

    /// 主色的候选、正在用的那个交给外观页（「主色」那一项照它画、开色块列表；跟随头像的选了写那个人格）。
    fn share_seeds(&mut self) {
        let colors = self.choice().colors;
        let persona = (colors == "persona")
            .then(|| self.seen().theme_persona().map(|p| p.id.clone()))
            .flatten();
        if let Some(settings) = &mut self.boxes.settings {
            settings.more.seeds.clone_from(&self.screen.theme_seeds);
            settings.more.seed = self.screen.theme_seed;
            settings.more.following = matches!(colors.as_str(), "persona" | "wallpaper");
            settings.more.seed_persona = persona;
            // 配置页里改了人格的主色：重读人格列表，照新的换颜色。
            if std::mem::take(&mut settings.personas_changed) {
                self.link.core.send(crate::core::Command::ListPersonas);
            }
        }
    }

    /// 开着配色的列表：引导「主题」那一步的、配置页外观页的、`/theme` 的框。
    fn colors_list_open(&self) -> bool {
        let colors = |c: &crate::settings::pages::choose::Choose| c.key == "tui.theme";
        let oobe = self
            .boxes
            .oobe
            .as_ref()
            .and_then(|o| o.theme.picking.as_ref());
        let settings = self.boxes.settings.as_ref().and_then(|s| match &s.popup {
            Some(crate::settings::popup::Popup::Choose(c)) => Some(c),
            _ => None,
        });
        let own = matches!(self.boxes.panel, Some(super::Panel::Theme { .. }));
        oobe.is_some_and(colors) || settings.is_some_and(colors) || own
    }

    /// 现在是不是跟随头像的配色（引导「主题」那一步选着的也算）。
    pub(super) fn following(&self) -> bool {
        self.choice().colors == "persona"
    }

    /// 能当主色的几个候选（好的在前）、正在用的那个：跟随壁纸的照找到的，自选的（`tui.theme_seed`）在候选里照它；跟随头像的
    /// 候选由核心给（2026-10-14 项目主人定），人格写了主题色的照它，没写的照打分最高的；引导里刚选、还没存的头像照门面现取
    /// （后台取，一张只取一次）。别的配色、都没有的、还没取好的是空的。
    fn wanted(&mut self) -> (Vec<u32>, Option<u32>) {
        if self.choice().colors == "wallpaper" {
            let seeds = self.screen.wallpaper.seeds().to_vec();
            let seed = choice::pick_seed(&seeds, self.screen.theme_pick);
            return (seeds, seed);
        }
        if !self.following() {
            return (Vec::new(), None);
        }
        match self.colors_source() {
            None => (Vec::new(), None),
            Some(Colors::Given { seed, seeds }) => {
                let seeds: Vec<u32> = seeds.iter().filter_map(|s| persona::hex(s)).collect();
                let seed = seed
                    .as_deref()
                    .and_then(persona::hex)
                    .or_else(|| seeds.first().copied());
                (seeds, seed)
            }
            Some(Colors::File(path)) => {
                let key = format!("file:{}", path.display());
                let seeds = self.screen.seeds.of(&key, &path).unwrap_or_default();
                let seed = seeds.first().copied();
                (seeds, seed)
            }
        }
    }

    /// 用哪个主色，顺手记下候选（外观页「主色」照它列）。
    pub(super) fn wanted_seed(&mut self) -> Option<u32> {
        let (seeds, seed) = self.wanted();
        self.screen.theme_seeds = seeds;
        seed
    }
}
