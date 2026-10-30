//! 辅助平面私用区的字（Nerd Font 的 md 那一族，第 15、16 平面）画到终端以前的保护（蓝图 `tui.md`「图标」第 2 条）。
//!
//! 这种字占几格，各家终端认得不一样：kitty 照一格画，有的（项目主人手机上经 ssh 用的那个）当两格。界面照一格排，
//! 终端当两格时：字后面接着写的那几格整体往右错一格，那一行以后只重写变了的格子，错开的字、半个字形就留在屏上
//! （2026-10-16 项目主人报：思考那一行往上推，时间线左边留下半个原子）。
//!
//! 包在后端外面一层，ratatui 交来的格子照这样送：
//! 1. 这种字右边那一格先写（字后面都跟着空格）：当一格画的终端那里是空格，当两格的被字的后半盖住；
//! 2. 这种字最后一个一个单独写，前面都显式挪光标：写完以后终端的光标在哪不要紧，别的格子不再接着它写；
//! 3. 这种字的格子变了（换成别的、或者换成它）、它右边那一格变了：相邻那一格跟着重写（当两格的终端里改了一半，
//!    整个字就没了；当一格画、字形宽出一截的终端里，右边那格留着半个字形）。
//!
//! 要知道相邻那一格现在是什么，这一层记着送出去过的每一格（[`Guarded::shadow`]）：照行列排的一块连续格子，
//! 和 ratatui 自己的缓冲一样大；清屏时放掉。

use std::collections::HashSet;

use ratatui::backend::{Backend, ClearType, WindowSize};
use ratatui::buffer::Cell;
use ratatui::layout::{Position, Size};
use unicode_width::UnicodeWidthStr;

/// kitty 图片占着的格子写的字（`U+10EEEE`，第 16 平面私用区）：一张图几百格，不归这一层管。
const PICTURE: char = '\u{10EEEE}';

/// 这一格写的是宽度认不准的字：第 15、16 平面私用区的（kitty 图片的占位字除外）。
pub fn shaky(symbol: &str) -> bool {
    symbol
        .chars()
        .next()
        .is_some_and(|c| matches!(u32::from(c), 0xF_0000..=0x10_FFFD) && c != PICTURE)
}

/// 包在后端外面的一层。
pub struct Guarded<B> {
    inner: B,
    /// 送出去过的每一格：相邻那一格要重写时照它写。
    shadow: Shadow,
    /// 现在写着宽度认不准的字的格子。
    glyphs: HashSet<(u16, u16)>,
}

impl<B> Guarded<B> {
    /// 包一层。
    pub fn new(inner: B) -> Self {
        Self {
            inner,
            shadow: Shadow::default(),
            glyphs: HashSet::new(),
        }
    }

    /// 里面的后端（直接写字节的地方要它）。
    pub fn inner_mut(&mut self) -> &mut B {
        &mut self.inner
    }

    /// 屏上清空了：记着的都不作数。
    fn forget(&mut self) {
        self.shadow = Shadow::default();
        self.glyphs.clear();
    }

    /// 这一格现在写着什么；没送过的是空白。
    fn now(&self, at: (u16, u16)) -> Cell {
        self.shadow.get(at).cloned().unwrap_or_default()
    }
}

/// 送出去过的整屏：照行列排的一块连续格子，第 `y` 行第 `x` 格在 `y × width + x`。
#[derive(Default)]
struct Shadow {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl Shadow {
    /// 这一格记着的；出了界的没有。
    fn get(&self, (x, y): (u16, u16)) -> Option<&Cell> {
        let (x, y) = (usize::from(x), usize::from(y));
        (x < self.width && y < self.height).then(|| &self.cells[y * self.width + x])
    }

    /// 至少放得下 `width` × `height`：放不下的照两边大的那个重排，记着的格子原样搬过去，新出来的是空白。
    fn fit(&mut self, width: usize, height: usize) {
        if width <= self.width && height <= self.height {
            return;
        }
        let (w, h) = (width.max(self.width), height.max(self.height));
        let mut cells = vec![Cell::default(); w * h];
        for (y, row) in self.cells.chunks_exact(self.width.max(1)).enumerate() {
            cells[y * w..y * w + self.width].clone_from_slice(row);
        }
        *self = Self {
            width: w,
            height: h,
            cells,
        };
    }

    /// 记下这一格（先 [`Shadow::fit`] 过，一定在界内）。
    fn set(&mut self, (x, y): (u16, u16), cell: Cell) {
        let i = usize::from(y) * self.width + usize::from(x);
        self.cells[i] = cell;
    }
}

impl<B: Backend> Backend for Guarded<B> {
    type Error = B::Error;

    fn draw<'a, I>(&mut self, content: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        let updates: Vec<(u16, u16, Cell)> = content.map(|(x, y, c)| (x, y, c.clone())).collect();
        let touched: HashSet<(u16, u16)> = updates.iter().map(|(x, y, _)| (*x, *y)).collect();
        // 记着的放不下这一帧的格子：照这一帧最远的那一格重排（清屏以后的头一帧差不多铺满整屏，一般只排这一回）。
        // 不问后端终端多大：ratatui 照固定区域画时不问，这一层也不多问一回。
        let far = updates.iter().fold((0, 0), |(w, h), (x, y, _)| {
            (w.max(usize::from(*x) + 1), h.max(usize::from(*y) + 1))
        });
        self.shadow.fit(far.0, far.1);
        // 先记下这一帧以前哪几格写着这种字，再照这一帧改记着的。
        let before = self.glyphs.clone();
        for (x, y, cell) in &updates {
            self.shadow.set((*x, *y), cell.clone());
            if shaky(cell.symbol()) {
                self.glyphs.insert((*x, *y));
            } else {
                self.glyphs.remove(&(*x, *y));
            }
        }
        // 第 3 条：相邻那一格跟着重写。
        let mut extra: HashSet<(u16, u16)> = HashSet::new();
        for (x, y, cell) in &updates {
            let at = (*x, *y);
            // 换上的是宽字的，它自己盖着右边那一格，写它就是重写那一格：不另写（写了反倒把宽字劈成两半）。
            let narrow = shaky(cell.symbol()) || cell.symbol().width() <= 1;
            if (before.contains(&at) || shaky(cell.symbol()))
                && narrow
                && let Some(right) = x.checked_add(1)
            {
                extra.insert((right, *y));
            }
            if let Some(left) = x.checked_sub(1)
                && self.glyphs.contains(&(left, *y))
            {
                extra.insert((left, *y));
            }
        }
        extra.retain(|at| !touched.contains(at));
        let mut cells: Vec<(u16, u16, Cell)> = updates;
        cells.extend(extra.into_iter().map(|(x, y)| (x, y, self.now((x, y)))));
        cells.sort_by_key(|(x, y, _)| (*y, *x));
        // 第 1、2 条：别的格子一起先写，这种字最后一个一个写。
        let (glyphs, plain): (Vec<_>, Vec<_>) =
            cells.into_iter().partition(|(_, _, c)| shaky(c.symbol()));
        self.inner.draw(plain.iter().map(|(x, y, c)| (*x, *y, c)))?;
        for (x, y, cell) in &glyphs {
            self.inner.draw(std::iter::once((*x, *y, cell)))?;
        }
        Ok(())
    }

    fn append_lines(&mut self, n: u16) -> Result<(), Self::Error> {
        self.inner.append_lines(n)
    }

    fn hide_cursor(&mut self) -> Result<(), Self::Error> {
        self.inner.hide_cursor()
    }

    fn show_cursor(&mut self) -> Result<(), Self::Error> {
        self.inner.show_cursor()
    }

    fn get_cursor_position(&mut self) -> Result<Position, Self::Error> {
        self.inner.get_cursor_position()
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> Result<(), Self::Error> {
        self.inner.set_cursor_position(position)
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        self.forget();
        self.inner.clear()
    }

    fn clear_region(&mut self, clear_type: ClearType) -> Result<(), Self::Error> {
        // 清掉一部分的，记着的哪几格还在说不准：都不作数（ratatui 只在往上插行时用它）。
        self.forget();
        self.inner.clear_region(clear_type)
    }

    fn size(&self) -> Result<Size, Self::Error> {
        self.inner.size()
    }

    fn window_size(&mut self) -> Result<WindowSize, Self::Error> {
        self.inner.window_size()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests;
