//! 用哪套主题（蓝图 `tui.md`「主题」第 2 到 4、6 条，2026-10-13 项目主人定）：配色（跟随头像、跟随壁纸、tokyonight、terminal）、明暗
//! （跟随终端、深色、浅色），手动选了深浅铺不铺底色。照这几样和终端的底色挑出一套出厂的颜色、要不要铺。终端自己的配置项
//! `tui.theme`、`tui.theme_mode`、`tui.theme_background`（终端软件包清单）。

use ratatui::style::Color;
use serde_json::Value;

use super::Palette;

/// 明暗。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// 跟随终端：照终端的底色挑，问不到的当深色。
    #[default]
    Auto,
    /// 深色。
    Dark,
    /// 浅色。
    Light,
}

impl Mode {
    /// 读配置里的写法；认不得的是 `None`。
    pub fn read(text: &str) -> Option<Self> {
        match text {
            "auto" => Some(Self::Auto),
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            _ => None,
        }
    }
}

/// 配色：配置里的写法，列表照这个先后。
pub const COLORS: [&str; 4] = ["persona", "wallpaper", "tokyonight", "terminal"];
/// 照一个主色生成一套的配色：跟随头像、跟随壁纸（2026-10-14 项目主人定，同一套算法）。
const SEEDED: [&str; 2] = ["persona", "wallpaper"];

/// 跟随壁纸自选的主色（2026-10-14 项目主人定：能从几个候选里挑；跟随头像的存在人格上）：`#rrggbb`，没写的照打分最高的。
pub const SEED_KEY: &str = "tui.theme_seed";

/// `config.get` 回应里自选的主色；没写的、写错的是 `None`。
pub fn chosen_seed(got: &Value) -> Option<u32> {
    got["items"][SEED_KEY]["value"]
        .as_str()
        .and_then(super::persona::hex)
}

/// 候选里挑一个：自选的在候选里照它；不在的（换了壁纸，和 Android 一样）、没选的照打分最高的那个。
pub fn pick_seed(seeds: &[u32], chosen: Option<u32>) -> Option<u32> {
    chosen
        .filter(|c| seeds.contains(c))
        .or_else(|| seeds.first().copied())
}

/// 主题的几个配置项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// 配色：[`COLORS`] 里的一种。
    pub colors: String,
    /// 明暗。
    pub mode: Mode,
    /// 手动选了深浅时铺底色（第 4 条：能关）。
    pub ground: bool,
}

impl Default for Choice {
    fn default() -> Self {
        Self {
            colors: "tokyonight".into(),
            mode: Mode::Auto,
            ground: true,
        }
    }
}

impl Choice {
    /// 这几项的键。
    pub const KEYS: [&str; 3] = ["tui.theme", "tui.theme_mode", "tui.theme_background"];

    /// 读 `config.get` 的回应：读到的、认得的盖上，别的照旧。
    pub fn apply(&mut self, got: &Value) {
        let value = |key: &str| &got["items"][key]["value"];
        if let Some(colors) = value("tui.theme").as_str().filter(|c| COLORS.contains(c)) {
            self.colors = colors.to_string();
        }
        if let Some(mode) = value("tui.theme_mode").as_str().and_then(Mode::read) {
            self.mode = mode;
        }
        if let Some(ground) = value("tui.theme_background").as_bool() {
            self.ground = ground;
        }
    }

    /// 用深色那一份：手动选的照选的，跟随终端的照终端的底色（`terminal_dark`），问不到的当深色。
    pub fn dark(&self, terminal_dark: Option<bool>) -> bool {
        match self.mode {
            Mode::Dark => true,
            Mode::Light => false,
            Mode::Auto => terminal_dark.unwrap_or(true),
        }
    }

    /// 出厂的哪一份：跟随头像的底子是 tokyonight（头像生成的那几格盖在它上面）。
    pub fn base(&self, dark: bool) -> &'static str {
        match (self.colors.as_str(), dark) {
            ("terminal", true) => "terminal",
            ("terminal", false) => "terminal-light",
            (_, true) => "tokyonight-night",
            (_, false) => "tokyonight-day",
        }
    }

    /// 铺不铺底色：手动选了深浅、开着；terminal 那一套跟着终端的配色走，不铺（第 4 条）。
    pub fn grounded(&self) -> bool {
        self.mode != Mode::Auto && self.ground && self.colors != "terminal"
    }
}

/// 挑好了的一套：出厂的名字（`/theme` 的提示写它）、颜色、要铺的底色和字色（不铺的是 `None`）。
#[derive(Debug, Clone)]
pub struct Picked {
    /// 出厂那一份的名字。
    pub name: String,
    /// 颜色。
    pub palette: Palette,
    /// 铺的底色、字色。
    pub ground: Option<(Color, Color)>,
}

/// 终端配色照问到的终端颜色生成（「终端配色」，H15）；别的配色、没问到的照主题文件。
fn terminal_or(base: &Palette, colors: &str) -> Palette {
    if colors == "terminal" {
        super::derived::terminal(base, super::terminal_colors::seen())
    } else {
        base.clone()
    }
}

/// 照配置项、终端的底色挑：出厂的那一份找不到的是 `None`。跟随头像的、有主色（`seed`）的照它生成一套盖在 tokyonight
/// 上（第 5 条）；还没有主色的（没头像、头像还没要到）照 tokyonight。终端配色的、起来时问到了终端颜色的，底色照它推
/// （「终端配色」）。
pub fn pick(
    choice: &Choice,
    terminal_dark: Option<bool>,
    themes: &[(String, Palette)],
    seed: Option<u32>,
) -> Option<Picked> {
    let dark = choice.dark(terminal_dark);
    let name = choice.base(dark);
    let (_, base) = themes.iter().find(|(n, _)| n == name)?;
    let (name, palette) = match seed.filter(|_| SEEDED.contains(&choice.colors.as_str())) {
        Some(seed) => {
            let shade = if dark { "night" } else { "day" };
            let palette = super::persona::overlay(base, seed, dark);
            (format!("{}-{shade}", choice.colors), palette)
        }
        None => (name.to_string(), terminal_or(base, &choice.colors)),
    };
    let ground = choice
        .grounded()
        .then_some((palette.background.0, palette.text.0))
        .filter(|(bg, _)| *bg != Color::Reset);
    Some(Picked {
        name,
        palette,
        ground,
    })
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;
    use serde_json::json;

    use super::{Choice, Mode, pick};
    use crate::theme::builtin;

    #[test]
    fn a_picked_main_color_holds_only_while_it_is_still_a_candidate() {
        // 2026-10-14 项目主人定：跟随壁纸自选的主色在候选里照它；换了壁纸候选整个换了，回到打分最高的那个（和 Android 一样）。
        let seeds = [0xd1_2b33, 0x36_b446];
        assert_eq!(super::pick_seed(&seeds, Some(0x36_b446)), Some(0x36_b446));
        assert_eq!(
            super::pick_seed(&seeds, Some(0x12_3456)),
            Some(0xd1_2b33),
            "不在候选里"
        );
        assert_eq!(super::pick_seed(&seeds, None), Some(0xd1_2b33), "没选");
        assert_eq!(super::pick_seed(&[], Some(0x36_b446)), None, "没有候选");
    }

    #[test]
    fn following_the_wallpaper_colors_like_following_an_avatar_with_the_same_seed() {
        // 2026-10-14 项目主人定：跟随壁纸和跟随头像同一套算法，同一个主色出一样的颜色；没找到壁纸（没有主色）的照 tokyonight。
        let themes = builtin().unwrap();
        let choice = |colors: &str| Choice {
            colors: colors.into(),
            mode: Mode::Dark,
            ground: true,
        };
        let wallpaper = pick(&choice("wallpaper"), None, &themes, Some(0x2f_ab34)).unwrap();
        let persona = pick(&choice("persona"), None, &themes, Some(0x2f_ab34)).unwrap();
        assert_eq!(wallpaper.name, "wallpaper-night");
        assert_eq!(wallpaper.palette.background, persona.palette.background);
        assert_eq!(wallpaper.palette.thought, persona.palette.thought);
        let missing = pick(&choice("wallpaper"), None, &themes, None).unwrap();
        assert_eq!(missing.name, "tokyonight-night");
    }

    #[test]
    fn the_settings_are_read_and_unknown_values_are_ignored() {
        let mut c = Choice::default();
        c.apply(&json!({"items": {
            "tui.theme": {"value": "terminal"},
            "tui.theme_mode": {"value": "light"},
            "tui.theme_background": {"value": false}
        }}));
        assert_eq!(
            (c.colors.as_str(), c.mode, c.ground),
            ("terminal", Mode::Light, false)
        );
        c.apply(
            &json!({"items": {"tui.theme": {"value": "pink"}, "tui.theme_mode": {"value": "dim"}}}),
        );
        assert_eq!(
            (c.colors.as_str(), c.mode),
            ("terminal", Mode::Light),
            "认不得的不动"
        );
    }

    #[test]
    fn auto_follows_the_terminal_and_only_a_chosen_mode_paints_the_ground() {
        // 2026-10-13 项目主人定：跟随终端照终端的底色挑深浅、不铺；手动选的铺这一套的底色，能关。
        let themes = builtin().unwrap();
        let auto = Choice::default();
        assert_eq!(
            pick(&auto, Some(false), &themes, None).unwrap().name,
            "tokyonight-day"
        );
        assert_eq!(
            pick(&auto, None, &themes, None).unwrap().name,
            "tokyonight-night",
            "问不到的当深色"
        );
        assert!(
            pick(&auto, Some(false), &themes, None)
                .unwrap()
                .ground
                .is_none(),
            "跟随终端不铺"
        );
        let light = Choice {
            mode: Mode::Light,
            ..Choice::default()
        };
        let picked = pick(&light, Some(true), &themes, None).unwrap();
        assert_eq!(picked.name, "tokyonight-day", "手动选的不看终端");
        assert_eq!(
            picked.ground,
            Some((Color::Rgb(0xe1, 0xe2, 0xe7), Color::Rgb(0x37, 0x60, 0xbf)))
        );
        let off = Choice {
            ground: false,
            ..light.clone()
        };
        assert!(
            pick(&off, None, &themes, None).unwrap().ground.is_none(),
            "关了不铺"
        );
        let terminal = Choice {
            colors: "terminal".into(),
            ..light
        };
        let picked = pick(&terminal, None, &themes, None).unwrap();
        assert_eq!(picked.name, "terminal-light");
        assert!(picked.ground.is_none(), "terminal 跟着终端的配色走，不铺");
    }

    #[test]
    fn following_the_avatar_overlays_tokyonight_once_there_is_a_seed() {
        let themes = builtin().unwrap();
        let persona = Choice {
            colors: "persona".into(),
            mode: Mode::Dark,
            ..Choice::default()
        };
        let none = pick(&persona, None, &themes, None).unwrap();
        assert_eq!(none.name, "tokyonight-night", "还没有主色：照 tokyonight");
        let green = pick(&persona, None, &themes, Some(0x2fab34)).unwrap();
        assert_eq!(green.name, "persona-night");
        assert_ne!(green.palette.accent, none.palette.accent);
        assert_eq!(
            green.ground.map(|g| g.0),
            Some(green.palette.background.0),
            "铺的是生成的那一套的底色"
        );
        let tokyonight = Choice {
            colors: "tokyonight".into(),
            ..persona
        };
        assert_eq!(
            pick(&tokyonight, None, &themes, Some(0x2fab34))
                .unwrap()
                .name,
            "tokyonight-night",
            "不是跟随头像的不看主色"
        );
    }
}
