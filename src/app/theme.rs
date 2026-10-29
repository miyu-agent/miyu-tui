//! 用哪套主题（蓝图 `tui.md`「主题」）：照 `tui.theme*` 的配置项（引导「主题」那一步选着的盖在上面）、起来时问到的终端
//! 底色、跟随头像的主色挑一套，设好颜色、要不要铺底色；`/theme` 在几种配色之间轮换，只管这一次启动。

use super::App;
use crate::oobe::Oobe;
use crate::oobe::persona::Colors;
use crate::theme::choice::{self, Choice, Mode};
use crate::theme::terminal_colors;

impl App {
    /// 起来时问到了终端的底色：照它重挑（跟随终端的才看它）。
    pub fn set_terminal_dark(&mut self, dark: Option<bool>) {
        self.screen.terminal_dark = dark;
        self.apply_theme();
    }

    /// 现在照哪几样挑：配置项，引导「主题」那一步选着的盖在上面（「第一次打开的引导」第 23a 条）。
    fn choice(&self) -> Choice {
        let mut choice = self.screen.theme_choice.clone();
        if let Some(preview) = &self.screen.theme_preview {
            choice.colors.clone_from(&preview.colors);
            choice.mode = preview.mode;
            choice.ground = preview.ground;
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
        // 引导刚关：挑过的主色拿住，人格列表重读回来以前不闪回旧的（`theme/seed_hold.rs`）。
        if preview.is_none()
            && let Some(before) = &self.screen.theme_preview
        {
            self.screen.seed_hold.closed(before.seed);
        }
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

    /// 窗口拿回焦点：选着跟随壁纸的再找一次；明暗自动、终端配色的记下这一拍过后再问一次终端的颜色（「主题」第 11 条）。
    pub(super) fn theme_focus(&mut self) {
        let choice = self.choice();
        if choice.colors == "wallpaper" {
            let dark = choice.dark(self.screen.terminal_dark);
            self.screen.wallpaper.look(dark, true);
        }
        if choice.mode == Mode::Auto || choice.colors == "terminal" {
            self.screen.ask_colors = true;
        }
    }

    /// 再问一次终端的颜色：读按键停下（回话不能让它读走），问完接着读；变了照新的重挑（终端配色照它重新生成、明暗自动
    /// 照它挑深浅）。没变的什么都不做。
    pub fn ask_colors_again(&mut self, reader: &crate::reader::Reader) {
        reader.pause();
        let changed = terminal_colors::ask_again();
        reader.resume();
        if changed {
            let dark = terminal_colors::seen().ground.map(|g| g.dark);
            self.set_terminal_dark(dark.or(self.screen.terminal_dark));
        }
    }

    /// 主色的候选、正在用的那个交给外观页（「主色」那一项照它画、开色块列表；跟随头像的选了写那个人格）。
    fn share_seeds(&mut self) {
        let colors = self.choice().colors;
        let persona = (colors == "persona")
            .then(|| self.seen().theme_persona().map(|p| p.id.clone()))
            .flatten();
        // 引导「主题」那一步的「主色」照它列、画色块（2026-10-15 项目主人：配色方案和主色）。
        if let Some(oobe) = &mut self.boxes.oobe {
            oobe.theme.seeds.clone_from(&self.screen.theme_seeds);
            oobe.theme.seed = self.screen.theme_seed;
        }
        if let Some(settings) = &mut self.boxes.settings {
            settings.more.seeds.clone_from(&self.screen.theme_seeds);
            settings.more.seed = self.screen.theme_seed;
            // 终端配色有候选（真彩色、问到了终端的颜色）才列「主色」（2026-10-19 项目主人定）。
            settings.more.following = matches!(colors.as_str(), "persona" | "wallpaper")
                || (colors == "terminal" && !self.screen.theme_seeds.is_empty());
            settings.more.terminal_colors = colors == "terminal";
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

    /// 能当主色的几个候选（好的在前）、正在用的那个：跟随壁纸的照找到的、终端配色的照终端 16 色挑的，自选的
    /// （`tui.theme_seed`）在候选里照它；跟随头像的
    /// 候选由核心给（2026-10-14 项目主人定），人格写了主题色的照它，没写的照打分最高的；引导里刚选、还没存的头像照门面现取
    /// （后台取，一张只取一次）。别的配色、都没有的、还没取好的是空的。
    fn wanted(&mut self) -> (Vec<u32>, Option<u32>) {
        let (seeds, seed) = self.wanted_by_config();
        // 引导「主题」那一步挑过的主色在候选里的，照它（当场换样子）；关了引导、人格列表还没回来的照拿住的。
        let picked = self
            .screen
            .theme_preview
            .as_ref()
            .and_then(|p| p.seed)
            .filter(|s| seeds.contains(s));
        let following = self.following();
        let seed = self
            .screen
            .seed_hold
            .seed((picked, seed), &seeds, following);
        (seeds, seed)
    }

    /// 同 [`App::wanted`]，不看引导里挑的：照配置项、人格的。
    fn wanted_by_config(&mut self) -> (Vec<u32>, Option<u32>) {
        let colors = self.choice().colors;
        let source = if colors == "persona" {
            self.colors_source()
        } else {
            None
        };
        match source {
            Some(Colors::Given { seed, seeds }) => {
                let persona = choice::PersonaSeeds {
                    seed: seed.as_deref(),
                    seeds: &seeds,
                };
                choice::seeds(&colors, &[], Some(persona), None)
            }
            None => choice::seeds(
                &colors,
                self.screen.wallpaper.seeds(),
                None,
                self.screen.theme_pick,
            ),
            // 引导里刚选、还没存的头像：照门面现取（后台取，一张只取一次），只有全屏有。
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
