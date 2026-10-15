//! 终端调色板 16 色真实的颜色（蓝图 `tui.md`「主题」第 10 条，2026-10-14 项目主人定）：进了全屏、开始读按键以前用 OSC 4 问
//! 一次，后面跟一句 DA1（不认 OSC 4 的终端先答 DA1，当场知道问不到），最多等 1 秒。问到了、又是真彩色的，终端配色里调色板的
//! 颜色也能照字节算：运行状态行的流光照真颜色算渐变，和模式字一个颜色。问不到的照旧加粗、变暗。
//!
//! 终端的字色、底色（OSC 10、11，`terminal-colorsaurus`）也在这里问：明暗「自动」照它挑深浅，终端配色的面板、差异这些底色
//! 照它推（`derived.rs`）。
//!
//! 窗口拿回焦点时再问一次（2026-10-15 项目主人定，设计 13 的 H15）：终端换了配色，跟着它的（明暗自动、终端配色）照新的
//! 重挑；问句和读回话在 `answer.rs`。

use std::sync::{PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;

use ratatui::style::Color;

use super::Depth;

mod answer;

/// 0 到 15 号的真颜色。
pub type Colors = [(u8, u8, u8); 16];

/// 最多等这么久。
const WAIT: Duration = Duration::from_secs(1);
/// 拿回焦点再问时最多等这么久：这一下读按键停着，等久了这时按的键会丢（起来时答过的终端几毫秒就答完）。
const WAIT_AGAIN: Duration = Duration::from_millis(300);

/// 终端自己的字色、底色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ground {
    /// 字色。
    pub fg: (u8, u8, u8),
    /// 底色。
    pub bg: (u8, u8, u8),
    /// 底比字暗（照 `terminal-colorsaurus` 的看法）。
    pub dark: bool,
}

/// 问到的：起来时问一次，拿回焦点时再问。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seen {
    /// 16 色，16 个都答了才有。
    pub sixteen: Option<Colors>,
    /// 字色、底色。
    pub ground: Option<Ground>,
    /// 显示得了真彩色（「主题」第 8 条）。
    pub truecolor: bool,
}

static SEEN: RwLock<Seen> = RwLock::new(Seen {
    sixteen: None,
    ground: None,
    truecolor: false,
});

fn read() -> RwLockReadGuard<'static, Seen> {
    SEEN.read().unwrap_or_else(PoisonError::into_inner)
}

fn write() -> RwLockWriteGuard<'static, Seen> {
    SEEN.write().unwrap_or_else(PoisonError::into_inner)
}

/// 问终端 16 色：16 个都答了才算；不是终端、不认的、测试里的是 `None`。
pub fn probe() -> Option<Colors> {
    if cfg!(test) {
        return None;
    }
    answer::ask(&answer::sixteen_query(), WAIT).and_then(|got| answer::sixteen(&got))
}

/// 问终端的字色、底色（OSC 10、11）：要在开始读按键之前问；不认的终端、问不到的、测试里的是 `None`。
pub fn probe_ground() -> Option<Ground> {
    if cfg!(test) {
        return None;
    }
    let options = terminal_colorsaurus::QueryOptions::default();
    let palette = terminal_colorsaurus::color_palette(options).ok()?;
    Some(Ground {
        fg: palette.foreground.scale_to_8bit(),
        bg: palette.background.scale_to_8bit(),
        dark: palette.theme_mode() == terminal_colorsaurus::ThemeMode::Dark,
    })
}

/// 装上问到的（启动时一次）。
pub fn install(sixteen: Option<Colors>, ground: Option<Ground>, depth: Depth) {
    *write() = Seen {
        sixteen,
        ground,
        truecolor: depth == Depth::True,
    };
}

/// 问到的 16 色、字色和底色、是不是真彩色：终端配色照它生成（`derived.rs`）。
pub fn seen() -> Seen {
    *read()
}

/// 窗口拿回焦点：再问一次字色、底色和 16 色（一次问全），问到的盖上，交回变了没有。要在读按键停着时问（回话不能让
/// 读按键的读走）。起来时一样都没问到的不问：不答的终端每次拿回焦点都白等；这次没答的照旧用上次的。
pub fn ask_again() -> bool {
    let before = seen();
    if cfg!(test) || (before.sixteen.is_none() && before.ground.is_none()) {
        return false;
    }
    answer::ask(&answer::all_query(), WAIT_AGAIN)
        .is_some_and(|got| update(answer::sixteen(&got), answer::ground(&got)))
}

/// 盖上问到的（`None` 的照旧），交回变了没有。
fn update(sixteen: Option<Colors>, ground: Option<Ground>) -> bool {
    let mut seen = write();
    let next = merged(*seen, sixteen, ground);
    let changed = next != *seen;
    *seen = next;
    changed
}

/// 这次问到的盖在上次的上面：没答的照旧。
fn merged(before: Seen, sixteen: Option<Colors>, ground: Option<Ground>) -> Seen {
    Seen {
        sixteen: sixteen.or(before.sixteen),
        ground: ground.or(before.ground),
        truecolor: before.truecolor,
    }
}

/// 一个颜色的真字节，流光照它算渐变：真彩色的原样；调色板 16 色的照问到的；别的（256 色 16 号以后、终端自己的前景色、
/// 没问到的）是 `None`。不是真彩色的也是 `None`：别的色深要把渐变换成最近的 256 色、16 色，一道光扫过去会跳成别的颜色。
pub fn rgb(color: Color) -> Option<(u8, u8, u8)> {
    let seen = seen();
    rgb_in(color, seen.sixteen.as_ref().filter(|_| seen.truecolor))
}

fn rgb_in(color: Color, real: Option<&Colors>) -> Option<(u8, u8, u8)> {
    if let Color::Rgb(r, g, b) = color {
        return Some((r, g, b));
    }
    let n = index(color)?;
    real.map(|colors| colors[n])
}

/// 调色板 16 色的序号。
pub(super) fn index(color: Color) -> Option<usize> {
    Some(match color {
        Color::Black => 0,
        Color::Red => 1,
        Color::Green => 2,
        Color::Yellow => 3,
        Color::Blue => 4,
        Color::Magenta => 5,
        Color::Cyan => 6,
        Color::Gray => 7,
        Color::DarkGray => 8,
        Color::LightRed => 9,
        Color::LightGreen => 10,
        Color::LightYellow => 11,
        Color::LightBlue => 12,
        Color::LightMagenta => 13,
        Color::LightCyan => 14,
        Color::White => 15,
        Color::Indexed(n) if n < 16 => usize::from(n),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::{merged, rgb_in};

    /// 16 色：第 4 号（蓝）是 `#4080ff`，别的是灰。
    fn reply() -> super::Colors {
        let mut colors = [(0, 0, 0); 16];
        for (n, slot) in (0u8..).zip(colors.iter_mut()) {
            *slot = (n * 8, n * 8, n * 8);
        }
        colors[4] = (0x40, 0x80, 0xff);
        colors
    }

    #[test]
    fn palette_colors_take_the_answered_bytes_and_others_stay_unknown() {
        let colors = reply();
        assert_eq!(rgb_in(Color::Blue, Some(&colors)), Some((0x40, 0x80, 0xff)));
        assert_eq!(
            rgb_in(Color::Indexed(4), Some(&colors)),
            Some((0x40, 0x80, 0xff))
        );
        assert_eq!(rgb_in(Color::Blue, None), None, "没问到的");
        assert_eq!(rgb_in(Color::Indexed(73), Some(&colors)), None);
        assert_eq!(rgb_in(Color::Rgb(1, 2, 3), None), Some((1, 2, 3)));
    }

    #[test]
    fn asking_again_keeps_what_was_not_answered() {
        let ground = |bg: u8, dark| super::Ground {
            fg: (200 - bg, 200 - bg, 200 - bg),
            bg: (bg, bg, bg),
            dark,
        };
        let before = super::Seen {
            sixteen: Some(reply()),
            ground: Some(ground(20, true)),
            truecolor: true,
        };
        assert_eq!(
            merged(before, Some(reply()), None),
            before,
            "一样的、没答的：没变"
        );
        let after = merged(before, None, Some(ground(180, false)));
        assert_eq!(after.ground, Some(ground(180, false)), "换了底色");
        assert_eq!(after.sixteen, Some(reply()), "16 色这次没答：照旧");
        assert!(after.truecolor);
    }
}
