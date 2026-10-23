//! 拖滚动条时按像素报鼠标（2026-10-23 项目主人：拖着不顺。量过每帧不到 1 毫秒，是鼠标按整格报，600 行的会话挪一格跳
//! 十几二十行；照推荐定「按像素拖」）：抓住滚动条时打开终端的 SGR 像素坐标（DEC 1016，kitty、ghostty、wezterm 认），
//! 松开就关；读到的像素照一格多大（起来时问图的那一次知道）换回格子，另交带小数的第几行给滚动条。不认的终端照旧按格报，
//! 这里看坐标没超出屏幕就当格子。

use std::io::{self, Write};

use ratatui::crossterm::event::MouseEvent;

/// 开关、一格几个像素、这一次打开以后认没认出像素坐标。
#[derive(Debug, Default)]
pub struct PixelMouse {
    /// 一格多宽、多高（像素）；起来时没问到的是 `None`，那就不开。
    pub cell: Option<(u32, u32)>,
    /// 这一拍要不要开（正抓着滚动条）。
    pub want: bool,
    /// 上一帧整屏几列、几行（`ui::draw` 记）：没认出像素以前，超出它的坐标才当像素。
    pub size: (u16, u16),
    on: bool,
    /// 打开以后收到过超出屏幕的坐标：这个终端认像素坐标，之后的都照像素读，到关掉为止。
    seen: bool,
}

impl PixelMouse {
    /// 照 `want` 开关；变了才写。
    ///
    /// # Errors
    ///
    /// 写不出去。
    pub fn sync(&mut self, out: &mut impl Write) -> io::Result<()> {
        let want = self.want && self.cell.is_some();
        if want != self.on {
            self.on = want;
            self.seen = false;
            write!(out, "{}", if want { "\x1b[?1016h" } else { "\x1b[?1016l" })?;
        }
        Ok(())
    }

    /// 退出、挂起时关掉。
    ///
    /// # Errors
    ///
    /// 写不出去。
    pub fn reset(&mut self, out: &mut impl Write) -> io::Result<()> {
        self.on = false;
        self.want = false;
        self.seen = false;
        write!(out, "\x1b[?1016l")
    }

    /// 开着时把像素坐标换回格子（屏幕 `cols` 列、`rows` 行），另交带小数的第几行；没开、这个终端照格子报的原样交回。
    pub fn as_cells(&mut self, event: MouseEvent) -> (MouseEvent, Option<f64>) {
        let (cols, rows) = self.size;
        let Some((w, h)) = self.cell.filter(|_| self.on) else {
            return (event, None);
        };
        if !self.seen && event.column < cols && event.row < rows {
            return (event, None);
        }
        self.seen = true;
        let (w, h) = (w.max(1), h.max(1));
        let cell = |px: u16, size: u32, limit: u16| {
            u16::try_from(u32::from(px) / size)
                .unwrap_or(u16::MAX)
                .min(limit.saturating_sub(1))
        };
        let y = f64::from(event.row) / f64::from(h);
        let converted = MouseEvent {
            column: cell(event.column, w, cols),
            row: cell(event.row, h, rows),
            ..event
        };
        (converted, Some(y))
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    use super::PixelMouse;

    fn drag(column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn pixels_turn_back_into_cells_only_while_on_and_once_seen() {
        let mut p = PixelMouse {
            cell: Some((10, 20)),
            size: (140, 40),
            ..PixelMouse::default()
        };
        let mut out = Vec::new();
        assert_eq!(p.as_cells(drag(1000, 300)).1, None, "没开照原样");
        p.want = true;
        p.sync(&mut out).expect("写得进");
        assert_eq!(out, b"\x1b[?1016h");
        // 第一下超出屏幕：认出是像素。
        let (e, y) = p.as_cells(drag(1005, 310));
        assert_eq!((e.column, e.row), (100, 15));
        assert_eq!(y, Some(15.5));
        // 认出以后，落在屏幕范围以内的数也照像素读。
        let (e, y) = p.as_cells(drag(30, 10));
        assert_eq!((e.column, e.row, y), (3, 0, Some(0.5)));
        p.want = false;
        out.clear();
        p.sync(&mut out).expect("写得进");
        assert_eq!(out, b"\x1b[?1016l");
        // 不认像素的终端：开着也一直在屏幕范围以内，照格子。
        p.want = true;
        p.sync(&mut out).expect("写得进");
        assert_eq!(p.as_cells(drag(100, 15)), (drag(100, 15), None));
        // 没问到一格多大的不开。
        let mut none = PixelMouse {
            want: true,
            ..PixelMouse::default()
        };
        out.clear();
        none.sync(&mut out).expect("写得进");
        assert!(out.is_empty());
    }
}
