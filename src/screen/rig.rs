//! 测试用的终端模型：画面写出去的字节喂给 vt100，和 ratatui 画好的缓冲逐格比。

use ratatui::buffer::{Buffer, CellWidth};
use ratatui::layout::{Position, Rect, Size};
use ratatui::text::Line;
use ratatui::widgets::Widget;

use super::wire::{Wire, frame};
use super::{HEAVY, Screen};
use crate::glyph_guard;

/// 宽度认不准的字（第 15 平面私用区的 md-atom）。
pub const ATOM: &str = "\u{f0768}";
/// 藏光标。
pub const HIDE: &[u8] = b"\x1b[?25l";
/// 显示光标。
pub const SHOW: &[u8] = b"\x1b[?25h";

/// 一块画面和它后面的终端模型。
pub struct Rig {
    pub screen: Screen<Wire>,
    pub model: vt100::Parser,
}

impl Rig {
    /// `width`×`height` 的终端，还没画过。
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            screen: Screen::new(Wire::new(width, height)),
            model: vt100::Parser::new(height, width, 0),
        }
    }

    /// 画一帧几行字，交回缓冲和写出去的字节；字节同时喂给模型。
    pub fn lines(&mut self, lines: &[(u16, &str)], cursor: Option<Position>) -> (Buffer, Vec<u8>) {
        self.paint(
            |buf| {
                for (y, text) in lines {
                    Line::raw(*text).render(Rect::new(0, *y, buf.area.width, 1), buf);
                }
            },
            cursor,
        )
    }

    pub fn paint(
        &mut self,
        paint: impl FnOnce(&mut Buffer),
        cursor: Option<Position>,
    ) -> (Buffer, Vec<u8>) {
        let (buf, bytes) = frame(&mut self.screen, |buf| {
            paint(buf);
            cursor
        });
        self.model.process(&bytes);
        (buf, bytes)
    }

    /// 终端换成 `width`×`height`。
    pub fn resize(&mut self, width: u16, height: u16) {
        self.screen.backend_mut().size = Size::new(width, height);
        self.model.screen_mut().set_size(height, width);
    }

    /// 模型屏上第 `y` 行的字：宽字的后半不写。
    pub fn shown(&self, y: u16) -> String {
        let screen = self.model.screen();
        let (_, cols) = screen.size();
        (0..cols)
            .filter_map(|x| screen.cell(y, x))
            .filter(|cell| !cell.is_wide_continuation())
            .map(|cell| {
                if cell.has_contents() {
                    cell.contents().to_string()
                } else {
                    " ".into()
                }
            })
            .collect()
    }

    /// 模型屏上每一格和缓冲里排的都对得上：图占着的格子不比（终端上留着原来的），重格子只比第一个字（模型一格只存
    /// 二十来个字节）；宽字在模型里也是宽字（带 VS16 的 emoji 除外，见下）。
    pub fn matches(&self, buf: &Buffer) {
        let screen = self.model.screen();
        for y in 0..buf.area.height {
            let mut x = 0;
            while x < buf.area.width {
                let cell = &buf[(x, y)];
                x += if glyph_guard::skipped(cell) {
                    1
                } else {
                    cell.cell_width().max(1)
                };
                if glyph_guard::skipped(cell) {
                    continue;
                }
                let at = x - cell.cell_width().max(1);
                let shown = screen.cell(y, at).unwrap();
                let got = if shown.has_contents() {
                    shown.contents()
                } else {
                    " "
                };
                if cell.symbol().len() > HEAVY {
                    assert_eq!(
                        got.chars().next(),
                        cell.symbol().chars().next(),
                        "第 {y} 行第 {at} 格"
                    );
                } else {
                    assert_eq!(
                        got,
                        cell.symbol(),
                        "第 {y} 行第 {at} 格；整行「{}」",
                        self.shown(y)
                    );
                }
                if cell.symbol().contains('\u{FE0F}') && cell.cell_width() > 1 {
                    // 带 VS16 的 emoji：模型（vt100）照一格画、不清后面那一格，正是 ratatui 多写那一格要对付的终端。
                    // 后面那一格得是空的（写过了），不能留着旧字。
                    let next = screen.cell(y, at + 1).unwrap();
                    assert!(
                        !next.has_contents() || next.contents() == " ",
                        "第 {y} 行第 {at} 格的 emoji 后面留着「{}」",
                        next.contents()
                    );
                } else {
                    assert_eq!(
                        shown.is_wide(),
                        cell.cell_width() > 1,
                        "第 {y} 行第 {at} 格宽不宽"
                    );
                }
            }
        }
    }
}

/// 缓冲里排的第 `y` 行：宽字盖住的后面那几格不写。
pub fn intended(buf: &Buffer, y: u16) -> String {
    let mut out = String::new();
    let mut x = 0;
    while x < buf.area.width {
        let cell = &buf[(x, y)];
        out.push_str(cell.symbol());
        x += cell.cell_width().max(1);
    }
    out
}

/// 字节里所有挪光标（`CSI 行;列 H`）挪到的地方，从 0 数。
pub fn moves(bytes: &[u8]) -> Vec<(u16, u16)> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = Vec::new();
    for part in text.split("\x1b[").skip(1) {
        let Some(end) = part.find(|c: char| !(c.is_ascii_digit() || c == ';')) else {
            continue;
        };
        if part[end..].starts_with('H')
            && let Some((row, col)) = part[..end].split_once(';')
        {
            out.push((
                col.parse::<u16>().unwrap() - 1,
                row.parse::<u16>().unwrap() - 1,
            ));
        }
    }
    out
}

/// 一段字节里有没有 `needle`。
pub fn has(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|w| w == needle)
}

/// 随机的一帧：每一行的字、图占着的格子、右下角一个重格子（或者没有）。
pub struct Scene {
    pub rows: Vec<String>,
    pub skips: Vec<(u16, u16)>,
    pub heavy: Option<char>,
}

impl Scene {
    /// 照这一帧画进缓冲。
    pub fn paint(&self, buf: &mut Buffer) {
        let width = usize::from(buf.area.width);
        // 画面换小了的，出界的行、格子不画。
        for (y, text) in (0u16..buf.area.height).zip(&self.rows) {
            buf.set_stringn(0, y, text, width, ratatui::style::Style::default());
        }
        let area = buf.area;
        let inside = |&&(x, y): &&(u16, u16)| x < area.width && y < area.height;
        for &(x, y) in self.skips.iter().filter(inside) {
            // 图占着的格子只放在单独的窄字上（真的图也是这样：它盖着的格子由它自己写）。
            let narrow = |c: &ratatui::buffer::Cell| c.symbol().len() == 1;
            let lone = narrow(&buf[(x, y)]) && (x == 0 || buf[(x - 1, y)].cell_width() <= 1);
            if lone {
                buf[(x, y)].set_diff_option(ratatui::buffer::CellDiffOption::Skip);
            }
        }
        if let Some(letter) = self.heavy {
            let symbol: String = std::iter::once(letter)
                .chain(std::iter::repeat_n('\u{301}', 40))
                .collect();
            let at = (buf.area.width - 10, buf.area.height - 1);
            // 同图占着的格子：不放在宽字盖住的格子上（真的图也不会，那样缓冲本身就自相矛盾）。
            if buf[(at.0 - 1, at.1)].cell_width() > 1 {
                return;
            }
            buf[at].set_symbol(&symbol).set_diff_option(
                ratatui::buffer::CellDiffOption::ForcedWidth(std::num::NonZeroU16::MIN),
            );
        }
    }
}

/// 照固定种子一帧帧出随机的场景：每一帧大约四成的行换掉，`pieces` 是拼行用的字。
pub struct Scenes {
    rng: crate::rng::Rng,
    pieces: &'static [&'static str],
    rows: Vec<String>,
    width: u16,
}

impl Scenes {
    pub fn new(seed: u64, pieces: &'static [&'static str], width: u16, height: u16) -> Self {
        Self {
            rng: crate::rng::Rng::new(seed),
            pieces,
            rows: vec![String::new(); usize::from(height)],
            width,
        }
    }

    /// 一个 `[0, n)` 里的随机数。
    pub fn below(&mut self, n: u64) -> u64 {
        self.rng.between([0, n - 1])
    }

    /// 下一帧。
    pub fn next(&mut self) -> Scene {
        let count = self.pieces.len() as u64;
        for y in 0..self.rows.len() {
            if self.below(10) < 4 {
                let n = self.below(u64::from(self.width));
                self.rows[y] = (0..n)
                    .map(|_| self.pieces[self.below(count) as usize])
                    .collect();
            }
        }
        let height = self.rows.len() as u64;
        let skips = (0..self.below(5))
            .map(|_| {
                (
                    self.below(u64::from(self.width)) as u16,
                    self.below(height) as u16,
                )
            })
            .collect();
        let heavy = (self.below(10) < 7).then(|| if self.below(10) < 8 { 'H' } else { 'G' });
        Scene {
            rows: self.rows.clone(),
            skips,
            heavy,
        }
    }
}
