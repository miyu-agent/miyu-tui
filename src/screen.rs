//! 全屏的画面（蓝图 `tui.md`「每一帧」）：正在画的时候留着两份缓冲逐格比，和 ratatui 的 `Terminal` 一样只送变了的格子；
//! 闲下来把缓冲放掉，只留每一段的指纹，醒来的那一帧照指纹分段重写。
//!
//! ratatui 的 `Terminal` 常驻这一帧、上一帧两整份屏幕缓冲，一格 48 字节，160×45 的终端两份共约 0.69 MB，跟着终端的大小
//! 涨。这里画面用的 `Terminal` 后端是只画进缓冲、不往终端写的假后端（`Blank`），界面照旧收 `&mut Frame` 往里画：
//! - 活跃的时候留着它：每一帧先 `swap_buffers`（上一帧那份成了上一份），画进当前这份，`flush` 让 ratatui 照它原来那套
//!   逐格比，变了的格子 `Blank` 只记下位置；画面照这些位置连成截、往外扩（`spans.rs`），交给真的后端写。写出去的和原来的
//!   ratatui 一样。
//! - 终端是空的（刚起来、清过屏、换过大小）：新建一个，两份都是空的，画进去以后同样 `flush`，和空的上一份比，只写不是空白
//!   的格子，也和原来的 ratatui 一样。
//! - 上一帧画完过了 `screen_park_ms`（`layout.json`）没再画，就放掉（[`Screen::park`]）：照最后一帧算出每 [`SEGMENT`]
//!   格一段的指纹（160×45 的终端 1800 个 `u64`，14.4 kB），记下重格子和宽度认不准的字的位置，把两份缓冲放掉。两份各
//!   345.6 kB，比 glibc 定的大块门槛（128 KiB）大，放掉当场还给系统。
//! - 醒来的那一帧新建一个，画进去，逐段算指纹和放掉时记的比，变了的段连成截写出去；写完留着它，这一帧就是下一帧的上一份。
//!
//! 取舍：活跃的时候常驻两份缓冲，和原来一样，写出去的字节、一帧的耗时也一样；闲着的时候省下两份缓冲。醒来那一帧要重新
//! 向系统要页，比平常的一帧慢一点、按段写也比只写变了的格子多写几格。
//!
//! 重格子（字超过 [`HEAVY`] 字节的格子：sixel 的整张图、iTerm2 的一行图都塞在图左上角那一格里）：没变的不重写。活跃的
//! 时候 ratatui 比出来没变；醒来的那一帧照放掉时记的，同一位置是同样的重格子就跳过。
//!
//! 一截怎么写（宽字、图占着的格子、宽度认不准的字）见 `glyph_guard::spans`。光标照 ratatui `Terminal::draw` 原来的
//! 先后：格子写完，这一帧不显示的藏起来，显示的先显示再挪过去；不每帧先藏再显（输入法的预编辑挂在光标上，跟着藏/显会和
//! 输入框里的提示来回闪）。
//!
//! 终端换了大小、清屏：真的终端清屏，留着的缓冲、指纹和记着的位置全部作废，下一帧当终端是空的画（同 ratatui 全屏视口的
//! 自动换大小：变窄的先多清一次屏）。

use std::convert::Infallible;

use ratatui::backend::{Backend, ClearType, WindowSize};
use ratatui::buffer::Cell;
use ratatui::layout::{Position, Rect, Size};
use ratatui::{Frame, Terminal, TerminalOptions, Viewport};

use crate::glyph_guard;
use print::fingerprint;

pub mod idle;
mod print;
mod spans;

/// 一段几格（只在醒来的那一帧用）。段越短，醒来那一帧写的越少，放掉时记的指纹越多。量过（2026-11-23 LW-17，160×45，
/// 每一帧都照指纹分段写时）：转圈一帧中位 16 格一段约 355 字节、8 格 308、4 格 272（只送变了的格子约 205）。取 4 格：
/// 160 宽一行 40 段，整屏 1800 个指纹、14.4 kB。
pub const SEGMENT: usize = 4;

/// 一格的字超过这么多字节算重格子。正常的字素最长的是带肤色的多人 emoji 组合，三十来个字节；kitty 的占位字连三个
/// 变音符号十个字节；sixel、iTerm2 的图是几 KB 起。64 在两头中间，留着一倍的余地。
pub const HEAVY: usize = 64;

/// 全屏的画面：往 `backend` 写。
pub struct Screen<B> {
    backend: B,
    /// 终端上现在是什么。
    shown: Shown,
    /// 放掉时每一段的指纹，一行接一行（只在 [`Shown::Parked`] 时作数）。
    segments: Vec<u64>,
    /// 放掉时的重格子：（行、列）和这一格的指纹，照行列排好。活跃的时候不用。
    heavy: Vec<((u16, u16), u64)>,
    /// 上一帧写着宽度认不准的字的格子（行、列），照行列排好：这种字换成别的时，右边那一格也得跟着重写。
    glyphs: Vec<(u16, u16)>,
    /// 上一帧照多大画的；还没画过是 `None`。
    size: Option<Size>,
}

impl<B: Backend> Screen<B> {
    /// 一块还没画过的画面：第一帧整屏都写。
    pub const fn new(backend: B) -> Self {
        Self {
            backend,
            shown: Shown::Blank,
            segments: Vec::new(),
            heavy: Vec::new(),
            glyphs: Vec::new(),
            size: None,
        }
    }

    /// 真的后端（直接写字节的地方要它）。
    pub const fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// 现在留着缓冲没有（活跃）。
    #[cfg(test)]
    pub const fn active(&self) -> bool {
        matches!(self.shown, Shown::Kept(_))
    }

    /// 清屏：真的终端清掉，记着的全部作废，下一帧和空的比。
    ///
    /// # Errors
    /// 写终端失败。
    pub fn clear(&mut self) -> Result<(), B::Error> {
        self.forget();
        self.backend.clear_region(ClearType::All)
    }

    /// 终端刚清空：留着的缓冲、记着的全部作废。
    fn forget(&mut self) {
        self.shown = Shown::Blank;
        self.heavy.clear();
        self.glyphs.clear();
    }

    /// 闲下来了：照最后一帧记下每一段的指纹、重格子、宽度认不准的字的位置，把缓冲放掉。已经放掉的什么都不做。
    pub fn park(&mut self) {
        if !matches!(self.shown, Shown::Kept(_)) {
            return;
        }
        let Shown::Kept(mut kept) = std::mem::replace(&mut self.shown, Shown::Parked) else {
            return;
        };
        let buffer = kept.current_buffer_mut();
        let width = usize::from(buffer.area.width).max(1);
        self.heavy.clear();
        self.glyphs.clear();
        self.segments.clear();
        for (y, row) in (0u16..).zip(buffer.content.chunks(width)) {
            self.segments.extend(row.chunks(SEGMENT).map(fingerprint));
            record(y, row, &mut self.heavy, &mut self.glyphs);
        }
    }

    /// 画一帧：`render` 往这一帧里画，交回光标要显示在哪（`None` 藏起来）。
    ///
    /// # Errors
    /// 问终端大小、写终端失败。
    pub fn draw(
        &mut self,
        render: impl FnOnce(&mut Frame) -> Option<Position>,
    ) -> Result<(), B::Error> {
        let size = self.backend.size()?;
        if let Some(before) = self.size.filter(|before| *before != size) {
            // 换了大小：同 ratatui 全屏视口自动换大小（`Terminal::resize`），变窄的先清一次屏（防折行），再清一次。
            if size.width < before.width {
                self.backend.clear_region(ClearType::All)?;
            }
            self.backend.clear_region(ClearType::All)?;
            self.forget();
        }
        self.size = Some(size);
        let cursor = match std::mem::replace(&mut self.shown, Shown::Blank) {
            Shown::Kept(mut kept) => {
                kept.swap_buffers();
                self.active_frame(kept, render)?
            }
            // 终端是空的（刚起来、清过屏、换过大小）：同 ratatui，和空的上一份比，只写不是空白的格子。
            Shown::Blank => self.active_frame(scratch(size), render)?,
            Shown::Parked => self.wake_frame(size, render)?,
        };
        match cursor {
            None => self.backend.hide_cursor()?,
            Some(at) => {
                self.backend.show_cursor()?;
                self.backend.set_cursor_position(at)?;
            }
        }
        self.backend.flush()
    }

    /// 活跃的一帧：画进 `kept` 当前那一份，ratatui 和上一份逐格比出变了的格子，连成截写出去；写完留着。
    fn active_frame(
        &mut self,
        mut kept: Terminal<Blank>,
        render: impl FnOnce(&mut Frame) -> Option<Position>,
    ) -> Result<Option<Position>, B::Error> {
        // 不走 `Terminal::draw`：它画完还要藏光标、挪光标、交回 `CompletedFrame`，这些都由这一层自己做。
        let cursor = render(&mut kept.get_frame());
        kept.backend_mut().changed.clear();
        let Ok(()) = kept.flush();
        let mut changed = std::mem::take(&mut kept.backend_mut().changed);
        let buffer = kept.current_buffer_mut();
        let width = usize::from(buffer.area.width).max(1);
        let mut spans = Vec::new();
        let rows = changed.chunk_by(|a, b| a.0 == b.0);
        for group in rows {
            let y = group[0].0;
            let start = usize::from(y) * width;
            let row = &buffer.content[start..start + width];
            let raw = spans::cells(group.iter().map(|(_, x)| usize::from(*x)));
            let was = |x: usize| {
                u16::try_from(x).is_ok_and(|x| self.glyphs.binary_search(&(y, x)).is_ok())
            };
            for (s, e) in spans::spans(row, raw, was) {
                spans.push((y, s, &row[usize::from(s)..usize::from(e)]));
            }
        }
        // 重格子：ratatui 比出来没变的（不在变了的格子里），终端上还是它，不重写。
        let changed_at = |x: u16, y: u16| changed.binary_search(&(y, x)).is_ok();
        let keep = |x: u16, y: u16, cell: &Cell| cell.symbol().len() <= HEAVY || changed_at(x, y);
        // 带 VS16 的 emoji 盖住的格子：ratatui 比出来变了的照写（同它）。
        let trail = |x: u16, y: u16, _: &Cell| changed_at(x, y);
        glyph_guard::spans(&mut self.backend, &spans, keep, trail)?;
        // 宽度认不准的字记着的位置照变了的格子改：变了的格子（连它盖住的）先去掉，这一帧写着这种字的再记上。
        for &(y, x) in &changed {
            let cell = &buffer[(x, y)];
            let reach = x.saturating_add(spans::reach(cell));
            self.glyphs
                .retain(|&(gy, gx)| gy != y || gx < x || gx >= reach);
            if glyph_guard::shaky(cell.symbol())
                && let Err(at) = self.glyphs.binary_search(&(y, x))
            {
                self.glyphs.insert(at, (y, x));
            }
        }
        changed.clear();
        kept.backend_mut().changed = changed;
        self.shown = Shown::Kept(kept);
        Ok(cursor)
    }

    /// 醒来的一帧：新建缓冲画进去，照放掉时记的指纹分段比，变了的段连成截写出去；写完留着缓冲。
    fn wake_frame(
        &mut self,
        size: Size,
        render: impl FnOnce(&mut Frame) -> Option<Position>,
    ) -> Result<Option<Position>, B::Error> {
        let mut kept = scratch(size);
        let cursor = render(&mut kept.get_frame());
        let buffer = kept.current_buffer_mut();
        let width = usize::from(size.width).max(1);
        let per_row = width.div_ceil(SEGMENT);
        let mut spans = Vec::new();
        let mut glyphs = Vec::new();
        let mut dirty = Vec::with_capacity(per_row);
        for ((y, row), prints) in (0u16..)
            .zip(buffer.content.chunks(width))
            .zip(self.segments.chunks(per_row))
        {
            dirty.clear();
            dirty.extend(
                row.chunks(SEGMENT)
                    .zip(prints)
                    .map(|(cells, last)| *last != fingerprint(cells)),
            );
            let was = |x: usize| {
                u16::try_from(x).is_ok_and(|x| self.glyphs.binary_search(&(y, x)).is_ok())
            };
            for (s, e) in spans::spans(row, spans::segments(&dirty, width), was) {
                spans.push((y, s, &row[usize::from(s)..usize::from(e)]));
            }
            let mut heavy = Vec::new();
            record(y, row, &mut heavy, &mut glyphs);
        }
        // 重格子和放掉时同一位置的一样：终端上还是它，不重写。
        let before = &self.heavy;
        let keep = |x: u16, y: u16, cell: &Cell| {
            cell.symbol().len() <= HEAVY
                || before
                    .binary_search_by_key(&(y, x), |(at, _)| *at)
                    .map_or(true, |i| {
                        before[i].1 != fingerprint(std::slice::from_ref(cell))
                    })
        };
        // 带 VS16 的 emoji 盖住的格子：不知道上一帧那里写着什么，一律写（ratatui 照它的规矩只写变了的）。
        let trail = |_: u16, _: u16, _: &Cell| true;
        glyph_guard::spans(&mut self.backend, &spans, keep, trail)?;
        self.glyphs = glyphs;
        self.heavy.clear();
        self.shown = Shown::Kept(kept);
        Ok(cursor)
    }
}

/// 第 `y` 行（`row`）里写得出去的格子（宽字盖住的、图占着的不算）里的重格子和宽度认不准的字，记进 `heavy`、`glyphs`。
/// 先只看字（快），这一行有这两种才照宽字一格格走一遍（算宽度慢）。
fn record(y: u16, row: &[Cell], heavy: &mut Vec<((u16, u16), u64)>, glyphs: &mut Vec<(u16, u16)>) {
    let special = |cell: &Cell| cell.symbol().len() > HEAVY || glyph_guard::shaky(cell.symbol());
    if !row.iter().any(special) {
        return;
    }
    for (x, _, cell, _) in glyph_guard::shown(y, 0, row, &|_, _, _| false) {
        if cell.symbol().len() > HEAVY {
            heavy.push(((y, x), fingerprint(std::slice::from_ref(cell))));
        } else if glyph_guard::shaky(cell.symbol()) {
            glyphs.push((y, x));
        }
    }
}

/// 终端上现在是什么。
enum Shown {
    /// 空的（刚起来、清过屏、换过大小）：下一帧新建缓冲，和空的上一份逐格比。
    Blank,
    /// 放掉缓冲时的那一帧，只记着每一段的指纹：下一帧（醒来）照指纹分段比。
    Parked,
    /// 活跃：留着的 `Terminal`，当前那一份是上一帧画的。
    Kept(Terminal<Blank>),
}

/// 照 `size` 新建一个只画进缓冲的 `Terminal`，两份缓冲都是空的。固定视口：建的时候不问后端大小、不问光标在哪
/// （ratatui 的 `Terminal::with_options`），假后端也就从不出错。
fn scratch(size: Size) -> Terminal<Blank> {
    let blank = Blank {
        size,
        changed: Vec::new(),
    };
    let options = TerminalOptions {
        viewport: Viewport::Fixed(Rect::from((Position::ORIGIN, size))),
    };
    let Ok(terminal) = Terminal::with_options(blank, options);
    terminal
}

/// 只画进缓冲的假后端：往终端写的一律不做，大小交回给定的；ratatui 逐格比完交来的格子只记下位置（行、列，从上到下、
/// 从左到右）。
struct Blank {
    size: Size,
    changed: Vec<(u16, u16)>,
}

impl Backend for Blank {
    type Error = Infallible;

    fn draw<'a, I>(&mut self, content: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        self.changed.extend(content.map(|(x, y, _)| (y, x)));
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn get_cursor_position(&mut self) -> Result<Position, Self::Error> {
        Ok(Position::ORIGIN)
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, _position: P) -> Result<(), Self::Error> {
        Ok(())
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn clear_region(&mut self, _clear_type: ClearType) -> Result<(), Self::Error> {
        Ok(())
    }

    fn size(&self) -> Result<Size, Self::Error> {
        Ok(self.size)
    }

    fn window_size(&mut self) -> Result<WindowSize, Self::Error> {
        Ok(WindowSize {
            columns_rows: self.size,
            pixels: Size::ZERO,
        })
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod park_tests;
#[cfg(test)]
mod rig;
#[cfg(test)]
mod segment_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub mod wire;
