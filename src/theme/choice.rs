//! 用哪套主题（蓝图 `tui.md`「主题」第 2 到 4、6 条，2026-10-13 项目主人定）：配色（跟随头像、跟随壁纸、tokyonight、terminal）、明暗
//! （自动、深色、浅色），铺不铺底色。照这几样和终端的底色挑出一套出厂的颜色、要不要铺。终端自己的配置项
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
/// 照一个主色生成一套的配色：跟随头像、跟随壁纸（2026-10-14 项目主人定，同一套算法）；终端配色真彩色、问到了终端颜色的
/// 也是（2026-10-19 项目主人定，主色从终端 16 色里挑）。
pub const SEEDED: [&str; 3] = ["persona", "wallpaper", "terminal"];

/// 铺底色那一项的键：配色是终端配色时引导、配置页都不列它（2026-10-15 项目主人定）。
pub const GROUND_KEY: &str = "tui.theme_background";

/// 跟随壁纸、终端配色自选的主色（2026-10-14 项目主人定：能从几个候选里挑；跟随头像的存在人格上；终端配色 2026-10-19
/// 共这一项）：`#rrggbb`，没写的、不在候选里的照打分最高的。
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
    /// 铺底色（第 4 条：能关）。
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

    /// 铺不铺底色：开着就铺（明暗自动的铺照终端挑出的那一套，2026-10-15 项目主人定）；terminal 那一套跟着终端的配色走，
    /// 不铺（第 4 条）。
    pub fn grounded(&self) -> bool {
        self.ground && self.colors != "terminal"
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

/// 终端配色的主色候选（「主色」第 4a 条）：照起来时、拿回焦点时问到的终端颜色挑；不是真彩色、没问到的是空的。
pub fn terminal_seeds() -> Vec<u32> {
    super::derived::seeds(super::terminal_colors::seen())
}

/// 跟随头像的配色照谁取色：人格自选的主题色、头像的候选主色（`#rrggbb`，好的在前）。
#[derive(Debug, Clone, Copy)]
pub struct PersonaSeeds<'a> {
    /// 自选的主题色，没选的是 `None`。
    pub seed: Option<&'a str>,
    /// 头像的候选。
    pub seeds: &'a [String],
}

/// 照配色挑主色的候选和用哪个（「主色」第 4、4a、5 条）：跟随壁纸的照找到的（`wallpaper`）、终端配色的照问到的终端颜色，
/// 自选的（`chosen`，`tui.theme_seed`）在候选里照它；跟随头像的照人格（`persona`），写了主题色的照它，没写的照打分最高的。
/// 别的配色、跟随头像却没有人格的是空的。只照这几样算，不看界面的状态：全屏、shellhook 都照它挑。
pub fn seeds(
    colors: &str,
    wallpaper: &[u32],
    persona: Option<PersonaSeeds<'_>>,
    chosen: Option<u32>,
) -> (Vec<u32>, Option<u32>) {
    match colors {
        "wallpaper" | "terminal" => {
            let seeds = match colors {
                "wallpaper" => wallpaper.to_vec(),
                _ => terminal_seeds(),
            };
            let seed = pick_seed(&seeds, chosen);
            (seeds, seed)
        }
        "persona" => match persona {
            None => (Vec::new(), None),
            Some(p) => {
                let seeds: Vec<u32> = p
                    .seeds
                    .iter()
                    .filter_map(|s| super::persona::hex(s))
                    .collect();
                let seed = p
                    .seed
                    .and_then(super::persona::hex)
                    .or_else(|| seeds.first().copied());
                (seeds, seed)
            }
        },
        _ => (Vec::new(), None),
    }
}

/// 照配置项、终端的底色挑：出厂的那一份找不到的是 `None`。跟随头像、跟随壁纸的、有主色（`seed`）的照它生成一套盖在
/// tokyonight 上（第 5 条）；还没有主色的（没头像、头像还没要到）照 tokyonight。终端配色的照问到的终端颜色：真彩色的拿
/// 打分最高的那个当默认主色同样生成、底换成终端的底；问不到、不是真彩色的照主题文件，底色照问到的推（「终端配色」）。
pub fn pick(
    choice: &Choice,
    terminal_dark: Option<bool>,
    themes: &[(String, Palette)],
    seed: Option<u32>,
) -> Option<Picked> {
    pick_in(
        choice,
        terminal_dark,
        themes,
        seed,
        super::terminal_colors::seen(),
    )
}

fn pick_in(
    choice: &Choice,
    terminal_dark: Option<bool>,
    themes: &[(String, Palette)],
    seed: Option<u32>,
    seen: super::terminal_colors::Seen,
) -> Option<Picked> {
    let dark = choice.dark(terminal_dark);
    let terminal = choice.colors == "terminal";
    // 终端配色照现在问到的候选挑：还没挑过主色的（刚起来、候选还没交下来）也照打分最高的生成，不先闪一帧 16 色那一套；
    // 没有候选的（不是真彩色、没问到）照主题文件。
    let seed = if terminal {
        pick_seed(&super::derived::seeds(seen), seed)
    } else {
        seed.filter(|_| SEEDED.contains(&choice.colors.as_str()))
    };
    let name = match (seed, dark) {
        (Some(_), true) => "tokyonight-night",
        (Some(_), false) => "tokyonight-day",
        (None, _) => choice.base(dark),
    };
    let (_, base) = themes.iter().find(|(n, _)| n == name)?;
    let (name, palette) = match seed {
        Some(seed) => {
            let shade = if dark { "night" } else { "day" };
            let mut palette = super::persona::overlay(base, seed, dark);
            if terminal {
                palette = super::derived::seeded(palette, seen);
            }
            (format!("{}-{shade}", choice.colors), palette)
        }
        None if terminal => (name.to_string(), super::derived::terminal(base, seen)),
        None => (name.to_string(), base.clone()),
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
mod tests;
