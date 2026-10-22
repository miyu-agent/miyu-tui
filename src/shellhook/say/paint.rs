//! 印在 shell 里（图纸 `shellhook.md`「怎么印」）：不全屏，光标下面一块原地刷新的活动区，定下来的行印上去就留在滚动区。
//!
//! 每一帧先画进一块和终端一样宽的画布，照全屏那一套收拾（宽字后面那格、主题的统一换色、照色深降色，蓝图 `tui.md`
//! 「每一帧」），再一行行写成带颜色的字；刷新包在同步输出里，不闪（设计 22 第七节）。

use std::io::{self, Write};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::crossterm::style::{Attribute, Print, ResetColor, SetAttribute, SetStyle};
use ratatui::crossterm::terminal::{
    BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate,
};
use ratatui::crossterm::{cursor, queue};
use ratatui::prelude::IntoCrossterm;
use ratatui::style::Style;
use ratatui::text::Line;
use unicode_width::UnicodeWidthStr;

use crate::theme;

/// 活动区里写字的那一格：第几行、第几列。
pub type Caret = Option<(usize, u16)>;

/// 往终端写的那一头：活动区现在占了几行、光标停在它的第几行。
pub struct Painter<W: Write> {
    out: W,
    /// 活动区占的行数；没有活动区是 0。
    live: u16,
    /// 光标停在活动区的第几行：平常是最后一行的行尾，抽屉里写字时停在写到的地方（输入法的候选框跟着它）。
    at: u16,
    /// 写字的那一格亮着光标。
    shown: bool,
    /// 上一帧的活动区和光标：一样的不再画（重画时光标要回上面去，输入法的候选框跟着跳，2026-10-22 验收）。
    last: Option<(Vec<String>, Caret)>,
    depth: theme::Depth,
}

impl<W: Write> Painter<W> {
    /// 往 `out` 写，颜色照 `depth` 降。
    pub fn new(out: W, depth: theme::Depth) -> Self {
        Self {
            out,
            live: 0,
            at: 0,
            shown: false,
            last: None,
            depth,
        }
    }

    /// 换掉活动区：擦掉上一帧的，`settled` 印上去留下，`live` 画在下面、下一帧再换。
    ///
    /// # Errors
    ///
    /// 写终端失败。
    pub fn frame(&mut self, settled: &[String], live: &[String]) -> io::Result<()> {
        self.frame_at(settled, live, None)
    }

    /// 同 [`Painter::frame`]，`caret` 是活动区里写字的那一格（第几行、第几列）：光标停在那里亮着，终端的输入法候选框跟着
    /// 它（2026-10-22 验收：抽屉里写理由时候选框跑到右下角）。没有的照旧停在最后一行、不亮。
    ///
    /// # Errors
    ///
    /// 写终端失败。
    pub fn frame_at(
        &mut self,
        settled: &[String],
        live: &[String],
        caret: Caret,
    ) -> io::Result<()> {
        if settled.is_empty()
            && self
                .last
                .as_ref()
                .is_some_and(|(l, c)| l.as_slice() == live && *c == caret)
        {
            return Ok(());
        }
        self.last = Some((live.to_vec(), caret));
        queue!(self.out, BeginSynchronizedUpdate)?;
        self.erase()?;
        for line in settled {
            queue!(self.out, Print(line), Print("\r\n"))?;
        }
        for (i, line) in live.iter().enumerate() {
            if i > 0 {
                queue!(self.out, Print("\r\n"))?;
            }
            queue!(self.out, Print(line))?;
        }
        self.live = u16::try_from(live.len()).unwrap_or(u16::MAX);
        self.at = self.live.saturating_sub(1);
        match caret.and_then(|(row, col)| Some((u16::try_from(row).ok()?, col))) {
            Some((row, col)) if row < self.live => {
                if self.at > row {
                    queue!(self.out, cursor::MoveUp(self.at - row))?;
                }
                queue!(self.out, cursor::MoveToColumn(col), cursor::Show)?;
                self.at = row;
                self.shown = true;
            }
            _ if self.shown => {
                queue!(self.out, cursor::Hide)?;
                self.shown = false;
            }
            _ => {}
        }
        queue!(self.out, EndSynchronizedUpdate)?;
        self.out.flush()
    }

    /// 收尾：擦掉活动区，光标停在新的一行开头，交还给 shell。
    ///
    /// # Errors
    ///
    /// 写终端失败。
    pub fn finish(&mut self) -> io::Result<()> {
        queue!(self.out, BeginSynchronizedUpdate)?;
        self.erase()?;
        self.last = None;
        // 写字时亮着的光标关上：亮不亮由说话那一套收工时定。
        if std::mem::take(&mut self.shown) {
            queue!(self.out, cursor::Hide)?;
        }
        queue!(self.out, ResetColor, EndSynchronizedUpdate)?;
        self.out.flush()
    }

    /// 擦掉活动区：回到它的第一行开头，往下全清。
    fn erase(&mut self) -> io::Result<()> {
        if self.live == 0 {
            return Ok(());
        }
        queue!(self.out, cursor::MoveToColumn(0))?;
        if self.at > 0 {
            queue!(self.out, cursor::MoveUp(self.at))?;
        }
        queue!(self.out, Clear(ClearType::FromCursorDown))?;
        self.live = 0;
        self.at = 0;
        Ok(())
    }

    /// 一行行写成带颜色的字：先画进画布，照全屏那一套收拾。`draw` 往画布上画，画布高 `height`、宽 `width`。
    pub fn render(
        &self,
        width: u16,
        height: u16,
        draw: impl FnOnce(&mut ratatui::Frame),
    ) -> io::Result<Vec<String>> {
        if width == 0 || height == 0 {
            return Ok(Vec::new());
        }
        let mut canvas =
            Terminal::new(TestBackend::new(width, height)).map_err(io::Error::other)?;
        let completed = canvas
            .draw(|frame| {
                draw(frame);
                crate::ui::wide::tidy(frame.buffer_mut());
                theme::paint(frame.buffer_mut());
                theme::degrade(frame.buffer_mut(), self.depth);
            })
            .map_err(io::Error::other)?;
        Ok(ansi_lines(completed.buffer))
    }

    /// 画好的这几行（[`Line`]）照 [`Painter::render`] 收拾成带颜色的字。
    pub fn lines(&self, width: u16, lines: &[Line<'static>]) -> io::Result<Vec<String>> {
        let height = u16::try_from(lines.len()).unwrap_or(u16::MAX);
        self.render(width, height, |frame| {
            let area = frame.area();
            for (y, line) in (0..height).zip(lines) {
                frame
                    .buffer_mut()
                    .set_line(area.x, area.y + y, line, area.width);
            }
        })
    }
}

/// 画布一行一行写成带颜色的字：宽字只写一次（后面那格跳过），行尾没颜色的空格不写（写满一行终端会自己折行）。
pub fn ansi_lines(buffer: &Buffer) -> Vec<String> {
    let area = buffer.area;
    (area.y..area.y + area.height)
        .map(|y| {
            let cells: Vec<_> = (area.x..area.x + area.width)
                .filter_map(|x| buffer.cell((x, y)))
                .collect();
            let end = cells
                .iter()
                .rposition(|c| c.symbol() != " " || c.bg != ratatui::style::Color::Reset)
                .map_or(0, |i| i + 1);
            let mut out = String::new();
            let mut last: Option<Style> = None;
            let mut skip = 0;
            for cell in &cells[..end] {
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                let style = cell.style();
                if last != Some(style) {
                    // 换样子时先全部复位再设，不叠上一段的粗体、斜体。
                    let mut code = Vec::new();
                    let reset = queue!(
                        code,
                        SetAttribute(Attribute::Reset),
                        SetStyle(style.into_crossterm())
                    );
                    if reset.is_ok() {
                        out.push_str(&String::from_utf8_lossy(&code));
                    }
                    last = Some(style);
                }
                out.push_str(cell.symbol());
                skip = cell.symbol().width().saturating_sub(1);
            }
            if last.is_some() {
                out.push_str("\u{1b}[0m");
            }
            out
        })
        .collect()
}

#[cfg(test)]
mod tests;
