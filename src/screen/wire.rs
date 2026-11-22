//! 测试用的后端：照真的终端写字节（crossterm 的后端写进一块内存），终端多大由测试定。

use std::io;

use ratatui::backend::{Backend, ClearType, CrosstermBackend, WindowSize};
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Size};

use super::Screen;

/// 写进内存的终端，大小由测试定。
pub struct Wire {
    pub inner: CrosstermBackend<Vec<u8>>,
    pub size: Size,
    /// 写过的格子（列、行），照写的先后。
    pub written: Vec<(u16, u16)>,
}

impl Wire {
    /// `width`×`height` 的终端，还没写过字节。
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            inner: CrosstermBackend::new(Vec::new()),
            size: Size::new(width, height),
            written: Vec::new(),
        }
    }

    /// 拿走到现在为止写的字节。
    pub fn take(&mut self) -> Vec<u8> {
        std::mem::take(self.inner.writer_mut())
    }
}

impl Backend for Wire {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        let written = &mut self.written;
        self.inner
            .draw(content.inspect(|(x, y, _)| written.push((*x, *y))))
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.inner.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.inner.show_cursor()
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        Err(io::Error::other("测试的终端不回话"))
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.inner.set_cursor_position(position)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.inner.clear()
    }

    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        self.inner.clear_region(clear_type)
    }

    fn size(&self) -> io::Result<Size> {
        Ok(self.size)
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        Ok(WindowSize {
            columns_rows: self.size,
            pixels: Size::ZERO,
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// 经画面画一帧：`paint` 往缓冲里画，交回光标；交回这一帧画好的缓冲和这一帧写出去的字节。
pub fn frame(
    screen: &mut Screen<Wire>,
    paint: impl FnOnce(&mut Buffer) -> Option<Position>,
) -> (Buffer, Vec<u8>) {
    let mut drawn = None;
    screen
        .draw(|f| {
            let cursor = paint(f.buffer_mut());
            drawn = Some(f.buffer_mut().clone());
            cursor
        })
        .unwrap_or_else(|e| panic!("画得出来：{e}"));
    let bytes = screen.backend_mut().take();
    (drawn.unwrap_or_else(|| panic!("画过")), bytes)
}
