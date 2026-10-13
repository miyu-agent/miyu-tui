//! 用哪套主题（蓝图 `tui.md`「主题」）：照 `tui.theme*` 的配置项（引导「主题」那一步选着的盖在上面）、起来时问到的终端
//! 底色、跟随头像的主色挑一套，设好颜色、要不要铺底色；`/theme` 在几种配色之间轮换，只管这一次启动。

use super::App;
use crate::oobe::Oobe;
use crate::oobe::persona::Colors;
use crate::theme::choice::{self, COLORS, Choice};
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
        let preview = self.boxes.oobe.as_ref().and_then(Oobe::theme_preview);
        let changed = preview != self.screen.theme_preview;
        self.screen.theme_preview = preview;
        let seed = self.wanted_seed();
        if changed || seed != self.screen.theme_seed {
            self.screen.theme_seed = seed;
            self.apply_theme();
        }
    }

    /// 跟随头像的配色照谁取色：引导开着的照人格那一步那一位，不然照正在看的人格（`Seen::theme_persona`）。
    pub(super) fn colors_source(&self) -> Option<Colors> {
        if let Some(oobe) = &self.boxes.oobe {
            return oobe.persona.colors();
        }
        let row = self.seen().theme_persona()?;
        match (&row.seed, &row.avatar) {
            (Some(seed), _) => Some(Colors::Seed(seed.clone())),
            (None, Some(version)) => Some(Colors::Avatar {
                persona: row.id.clone(),
                version: version.clone(),
            }),
            (None, None) => None,
        }
    }

    /// 现在是不是跟随头像的配色（引导「主题」那一步选着的也算）。
    pub(super) fn following(&self) -> bool {
        self.choice().colors == "persona"
    }

    /// 跟随头像的主色：写了主题色的照写的，没写的从头像取（后台取，一张只取一次）；不是跟随头像的、都没有的、头像还没要到
    /// 还没取好的是 `None`。
    fn wanted_seed(&mut self) -> Option<u32> {
        if !self.following() {
            return None;
        }
        match self.colors_source()? {
            Colors::Seed(seed) => persona::hex(&seed),
            Colors::File(path) => {
                let key = format!("file:{}", path.display());
                self.screen.seeds.of(&key, &path)
            }
            Colors::Avatar { version, .. } => {
                let file = self.media.avatars.file(&version)?;
                self.screen.seeds.of(&version, &file)
            }
        }
    }

    /// 换下一种配色，只管这一次启动（「主题」第 6 条），提示「主题：名字」。
    pub(super) fn next_theme(&mut self) {
        let colors = &mut self.screen.theme_choice.colors;
        let at = COLORS.iter().position(|c| c == colors).unwrap_or(0);
        *colors = COLORS[(at + 1) % COLORS.len()].to_string();
        self.screen.theme_seed = self.wanted_seed();
        self.apply_theme();
        let text = self
            .config
            .text
            .theme_changed
            .replace("{name}", &self.screen.theme);
        self.hint(text, true);
    }
}
