//! 终端调色板 16 色真实的颜色（蓝图 `tui.md`「主题」第 10 条，2026-10-14 项目主人定）：进了全屏、开始读按键以前用 OSC 4 问
//! 一次，后面跟一句 DA1（不认 OSC 4 的终端先答 DA1，当场知道问不到），最多等 1 秒。问到了、又是真彩色的，终端配色里调色板的
//! 颜色也能照字节算：运行状态行的流光照真颜色算渐变，和模式字一个颜色。问不到的照旧加粗、变暗。
//!
//! 终端的字色、底色（OSC 10、11，`terminal-colorsaurus`）也在这里问：明暗「自动」照它挑深浅，终端配色的面板、差异这些底色
//! 照它推（`derived.rs`）。

use std::sync::OnceLock;
use std::time::Duration;

use ratatui::style::Color;

use super::Depth;

/// 0 到 15 号的真颜色。
pub type Colors = [(u8, u8, u8); 16];

/// 最多等这么久。
const WAIT: Duration = Duration::from_secs(1);

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

/// 起来时问到的。
#[derive(Debug, Clone, Copy)]
struct Seen {
    sixteen: Option<Colors>,
    ground: Option<Ground>,
    truecolor: bool,
}

static SEEN: OnceLock<Seen> = OnceLock::new();

/// 问终端 16 色：16 个都答了才算；不是终端、不认的、测试里的是 `None`。
pub fn probe() -> Option<Colors> {
    if cfg!(test) {
        return None;
    }
    ask(WAIT).and_then(|got| parse(&got))
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
    SEEN.get_or_init(|| Seen {
        sixteen,
        ground,
        truecolor: depth == Depth::True,
    });
}

/// 问到的 16 色、字色和底色：终端配色照它推底色。
pub fn seen() -> (Option<Colors>, Option<Ground>) {
    SEEN.get().map_or((None, None), |s| (s.sixteen, s.ground))
}

/// 一个颜色的真字节，流光照它算渐变：真彩色的原样；调色板 16 色的照问到的；别的（256 色 16 号以后、终端自己的前景色、
/// 没问到的）是 `None`。不是真彩色的也是 `None`：别的色深要把渐变换成最近的 256 色、16 色，一道光扫过去会跳成别的颜色。
pub fn rgb(color: Color) -> Option<(u8, u8, u8)> {
    let sixteen = SEEN
        .get()
        .filter(|s| s.truecolor)
        .and_then(|s| s.sixteen.as_ref());
    rgb_in(color, sixteen)
}

fn rgb_in(color: Color, real: Option<&Colors>) -> Option<(u8, u8, u8)> {
    if let Color::Rgb(r, g, b) = color {
        return Some((r, g, b));
    }
    let n = index(color)?;
    real.map(|colors| colors[n])
}

/// 调色板 16 色的序号。
fn index(color: Color) -> Option<usize> {
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

/// 终端的回话：`ESC ] 4 ; 序号 ; rgb:RRRR/GGGG/BBBB` 后面跟 BEL 或 `ESC \`，一个颜色一句。一个通道 1 到 4 位十六进制。
fn parse(got: &[u8]) -> Option<Colors> {
    let text = String::from_utf8_lossy(got);
    let mut found = [None; 16];
    for part in text.split("\x1b]4;").skip(1) {
        let end = part.find(['\x07', '\x1b']).unwrap_or(part.len());
        let Some((n, spec)) = part[..end].split_once(';') else {
            continue;
        };
        let (Ok(n), Some(color)) = (n.parse::<usize>(), spec_rgb(spec)) else {
            continue;
        };
        if let Some(slot) = found.get_mut(n) {
            *slot = Some(color);
        }
    }
    let mut colors = [(0, 0, 0); 16];
    for (slot, color) in colors.iter_mut().zip(found) {
        *slot = color?;
    }
    Some(colors)
}

/// `rgb:RR/GG/BB` 这类写法（`rgba:` 的透明度不要）。
fn spec_rgb(spec: &str) -> Option<(u8, u8, u8)> {
    let channels = spec
        .strip_prefix("rgb:")
        .or_else(|| spec.strip_prefix("rgba:"))?;
    let mut parts = channels.split('/').map(channel);
    Some((parts.next()??, parts.next()??, parts.next()??))
}

/// 一个通道：几位十六进制都换成 0 到 255。
fn channel(hex: &str) -> Option<u8> {
    let digits = u32::try_from(hex.len())
        .ok()
        .filter(|n| (1..=4).contains(n))?;
    let value = u32::from_str_radix(hex, 16).ok()?;
    let max = 16u32.pow(digits) - 1;
    u8::try_from((value * 255 + max / 2) / max).ok()
}

/// 终端答完了：DA1 的回话（`ESC [ ? … c`）到了。只有问的那边（Unix）用。
#[cfg(any(unix, test))]
fn answered(got: &[u8]) -> bool {
    got.windows(3)
        .position(|w| w == b"\x1b[?")
        .is_some_and(|at| got[at..].contains(&b'c'))
}

/// 写问句、读回话，直到 DA1 答了；超时、读不成、标准输入不是终端的是 `None`。照 `ratatui-image` 问图的协议的读法：
/// 轮询标准输入（macOS 轮询不了 `/dev/tty`）。
#[cfg(unix)]
fn ask(wait: Duration) -> Option<Vec<u8>> {
    use std::io::{IsTerminal, Write};
    use std::time::Instant;

    use rustix::event::{PollFd, PollFlags, Timespec, poll};
    use rustix::fd::AsFd;

    let stdin = std::io::stdin();
    if !stdin.is_terminal() {
        return None;
    }
    let mut query: String = (0..16).map(|n| format!("\x1b]4;{n};?\x1b\\")).collect();
    query.push_str("\x1b[c");
    let mut out = std::io::stdout();
    out.write_all(query.as_bytes()).ok()?;
    out.flush().ok()?;
    let fd = stdin.as_fd();
    let end = Instant::now() + wait;
    let mut got = Vec::new();
    let mut chunk = [0u8; 512];
    while !answered(&got) {
        let left = Timespec::try_from(end.checked_duration_since(Instant::now())?).ok()?;
        let mut fds = [PollFd::new(&fd, PollFlags::IN)];
        if poll(&mut fds, Some(&left)).ok()? == 0 {
            return None;
        }
        // 用 rustix 的读：std 的读自己攒着，轮询会以为没有了。
        let n = rustix::io::read(fd, &mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        got.extend_from_slice(&chunk[..n]);
    }
    Some(got)
}

#[cfg(not(unix))]
fn ask(_: Duration) -> Option<Vec<u8>> {
    None
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::{answered, parse, rgb_in};

    /// 照 kitty 的样子答 16 色：4 位十六进制、`ESC \` 收尾；第 4 号（蓝）是 `#4080ff`。
    fn reply() -> Vec<u8> {
        let mut got = String::new();
        for n in 0..16 {
            let (r, g, b) = if n == 4 {
                (0x40, 0x80, 0xff)
            } else {
                (n * 8, n * 8, n * 8)
            };
            got.push_str(&format!(
                "\x1b]4;{n};rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}\x1b\\"
            ));
        }
        got.push_str("\x1b[?62;22c");
        got.into_bytes()
    }

    #[test]
    fn the_sixteen_answers_are_read_whatever_the_digits_and_ending() {
        let colors = parse(&reply()).unwrap();
        assert_eq!(colors[4], (0x40, 0x80, 0xff));
        assert_eq!(colors[15], (120, 120, 120));
        // 两位十六进制、BEL 收尾（有的终端这么答）。
        let short: String = (0..16)
            .map(|n| format!("\x1b]4;{n};rgb:ff/00/80\x07"))
            .collect();
        assert_eq!(parse(short.as_bytes()).unwrap()[0], (255, 0, 128));
        // 少答一个的不算。
        let missing: String = (0..15)
            .map(|n| format!("\x1b]4;{n};rgb:ff/00/80\x07"))
            .collect();
        assert_eq!(parse(missing.as_bytes()), None);
        assert!(answered(&reply()));
        assert!(!answered(b"\x1b]4;0;rgb:ff/00/80\x07"));
    }

    #[test]
    fn kittys_own_reply_reads_into_sixteen_colors() {
        // 真 kitty（无头，2026-10-14）答的原样：一个通道 4 位、`ESC \\` 收尾，DA1 带着分号结尾。
        let got = "\x1b]4;0;rgb:4c4c/4c4c/4c4c\x1b\\\x1b]4;1;rgb:acac/8a8a/8c8c\x1b\\\x1b]4;2;rgb:9a9a/b3b3/8d8d\x1b\\\x1b]4;3;rgb:acac/a9a9/8a8a\x1b\\\x1b]4;4;rgb:c9c9/bebe/ffff\x1b\\\x1b]4;5;rgb:eded/b8b8/cdcd\x1b\\\x1b]4;6;rgb:c9c9/c3c3/dcdc\x1b\\\x1b]4;7;rgb:f0f0/f0f0/f0f0\x1b\\\x1b]4;8;rgb:9393/8f8f/9999\x1b\\\x1b]4;9;rgb:c4c4/9e9e/a0a0\x1b\\\x1b]4;10;rgb:b0b0/c9c9/a3a3\x1b\\\x1b]4;11;rgb:c4c4/c1c1/9e9e\x1b\\\x1b]4;12;rgb:a3a3/9e9e/c4c4\x1b\\\x1b]4;13;rgb:ffff/d8d8/e6e6\x1b\\\x1b]4;14;rgb:c9c9/c3c3/dcdc\x1b\\\x1b]4;15;rgb:e7e7/e7e7/e7e7\x1b\\\x1b[?62;52;c";
        let colors = parse(got.as_bytes()).unwrap();
        assert_eq!(colors[0], (0x4c, 0x4c, 0x4c));
        assert_eq!(colors[4], (0xc9, 0xbe, 0xff));
        assert_eq!(colors[15], (0xe7, 0xe7, 0xe7));
        assert!(answered(got.as_bytes()));
    }

    #[test]
    fn palette_colors_take_the_answered_bytes_and_others_stay_unknown() {
        let colors = parse(&reply()).unwrap();
        assert_eq!(rgb_in(Color::Blue, Some(&colors)), Some((0x40, 0x80, 0xff)));
        assert_eq!(
            rgb_in(Color::Indexed(4), Some(&colors)),
            Some((0x40, 0x80, 0xff))
        );
        assert_eq!(rgb_in(Color::Blue, None), None, "没问到的");
        assert_eq!(rgb_in(Color::Indexed(73), Some(&colors)), None);
        assert_eq!(rgb_in(Color::Rgb(1, 2, 3), None), Some((1, 2, 3)));
    }
}
