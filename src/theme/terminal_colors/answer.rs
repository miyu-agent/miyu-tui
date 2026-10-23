//! 问终端的颜色、读它的回话（从 `mod.rs` 分出来）：OSC 4 问 16 色，OSC 10、11 问字色、底色，后面跟一句 DA1（不认的终端
//! 先答 DA1，当场知道问不到）。照 `ratatui-image` 问图的协议的读法轮询标准输入（macOS 轮询不了 `/dev/tty`），Windows 照 crossterm 的按键事件拼回回话。

use std::time::Duration;

use super::{Colors, Ground};

/// 只问 16 色（起来时；字色、底色另由 `terminal-colorsaurus` 问）。
pub(super) fn sixteen_query() -> String {
    let mut query: String = (0..16).map(|n| format!("\x1b]4;{n};?\x1b\\")).collect();
    query.push_str("\x1b[c");
    query
}

/// 字色、底色、16 色一次问全（拿回焦点再问时）。
pub(super) fn all_query() -> String {
    let mut query = String::from("\x1b]10;?\x1b\\\x1b]11;?\x1b\\");
    query.push_str(&sixteen_query());
    query
}

/// 终端的回话：`ESC ] 4 ; 序号 ; rgb:RRRR/GGGG/BBBB` 后面跟 BEL 或 `ESC \`，一个颜色一句。一个通道 1 到 4 位十六进制。
/// 16 个都答了才算。
pub(super) fn sixteen(got: &[u8]) -> Option<Colors> {
    let text = String::from_utf8_lossy(got);
    let mut found = [None; 16];
    for part in text.split("\x1b]4;").skip(1) {
        let Some((n, spec)) = body(part).split_once(';') else {
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

/// 字色、底色的回话：`ESC ] 10 ; rgb:…`、`ESC ] 11 ; rgb:…`，两句都答了才算；深浅照 `terminal-colorsaurus` 的看法。
pub(super) fn ground(got: &[u8]) -> Option<Ground> {
    let text = String::from_utf8_lossy(got);
    let one = |head: &str| {
        let part = text.split(head).nth(1)?;
        spec_rgb(body(part))
    };
    let (fg, bg) = (one("\x1b]10;")?, one("\x1b]11;")?);
    Some(Ground {
        fg,
        bg,
        dark: dark(fg, bg),
    })
}

/// 一句回话到 BEL 或 `ESC \` 为止。
fn body(part: &str) -> &str {
    let end = part.find(['\x07', '\x1b']).unwrap_or(part.len());
    &part[..end]
}

/// 底比字暗：照 `terminal-colorsaurus` 的 `ColorPalette::theme_mode`（起来时照它判的，再问时一个判法）。
fn dark(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> bool {
    let light = |(r, g, b): (u8, u8, u8)| {
        let wide = |c: u8| u16::from(c) * 257;
        terminal_colorsaurus::Color::rgb(wide(r), wide(g), wide(b)).perceived_lightness()
    };
    let (fg, bg) = (light(fg), light(bg));
    if bg < fg {
        true
    } else {
        !(bg > fg || bg > 0.5)
    }
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
#[cfg(any(unix, windows, test))]
fn answered(got: &[u8]) -> bool {
    got.windows(3)
        .position(|w| w == b"\x1b[?")
        .is_some_and(|at| got[at..].contains(&b'c'))
}

/// 写问句、读回话，直到 DA1 答了；超时、读不成、标准输入不是终端的是 `None`。
#[cfg(unix)]
pub(super) fn ask(query: &str, wait: Duration) -> Option<Vec<u8>> {
    use std::io::{IsTerminal, Write};
    use std::time::Instant;

    use rustix::event::{PollFd, PollFlags, Timespec, poll};
    use rustix::fd::AsFd;

    let stdin = std::io::stdin();
    if !stdin.is_terminal() {
        return None;
    }
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

/// Windows：照 crossterm 的事件读回话（2026-10-23 项目主人在 Windows Terminal 里看 shellhook：跟随终端的配色问不到 16 色，
/// 退回主题文件里的「蓝」，粗体的蓝暗得难看）。终端的回话在 Windows 上一个字一个按键事件进来：照字拼回字节，`Esc` 是
/// ESC，`Ctrl+字母` 是那个控制字符（BEL 是 `Ctrl+G`）。不写 unsafe：等输入用 crossterm 的 `poll`。
#[cfg(windows)]
pub(super) fn ask(query: &str, wait: Duration) -> Option<Vec<u8>> {
    use std::io::{IsTerminal, Write};
    use std::time::Instant;

    use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

    if !std::io::stdout().is_terminal() {
        return None;
    }
    let mut out = std::io::stdout();
    out.write_all(query.as_bytes()).ok()?;
    out.flush().ok()?;
    let end = Instant::now() + wait;
    let mut got = Vec::new();
    while !answered(&got) {
        let left = end.checked_duration_since(Instant::now())?;
        if !event::poll(left).ok()? {
            return None;
        }
        let Event::Key(key) = event::read().ok()? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        match key.code {
            KeyCode::Esc => got.push(0x1b),
            KeyCode::Char(c)
                if key.modifiers.contains(KeyModifiers::CONTROL) && c.is_ascii_alphabetic() =>
            {
                got.push(c.to_ascii_lowercase() as u8 - b'a' + 1);
            }
            KeyCode::Char(c) => {
                let mut buf = [0u8; 4];
                got.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
            _ => {}
        }
    }
    Some(got)
}

#[cfg(not(any(unix, windows)))]
pub(super) fn ask(_: &str, _: Duration) -> Option<Vec<u8>> {
    None
}

#[cfg(test)]
mod tests;
