//! 问的时候预览块滚到哪（发布前第五批审查 T5-1，蓝图「配置页」第 43c 条）：要批的能力、要装的东西一行行写在预览块里，
//! 块的高度照窗口剩下的算。放不下的能滚，没滚到底（没看到最后一行）之前 `y`、`Enter` 不做。画的时候照块高写回露几行，
//! 按键照它滚、照它判看没看完。

/// 预览块露的那一窗。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AskView {
    /// 从第几行露起。
    pub top: usize,
    /// 一共几行；还没画过的是 0（窗口小得画不出预览块时也是 0，算没看完）。
    pub rows: usize,
    /// 一窗露几行。
    pub room: usize,
}

impl AskView {
    /// 下面还有几行没露出来。
    pub fn left(&self) -> usize {
        self.rows.saturating_sub(self.top + self.room)
    }

    /// 画过、最后一行露出来过。
    pub fn seen(&self) -> bool {
        self.rows > 0 && self.left() == 0
    }

    /// 往下（正）、往上（负）滚几行，滚不出头尾。
    pub fn scroll(&mut self, by: isize) {
        self.top = self
            .top
            .saturating_add_signed(by)
            .min(self.rows.saturating_sub(self.room));
    }

    /// 翻一窗。
    pub fn page(&mut self, down: bool) {
        let by = isize::try_from(self.room.max(1)).unwrap_or(1);
        self.scroll(if down { by } else { -by });
    }
}

#[cfg(test)]
mod tests {
    use super::AskView;

    #[test]
    fn an_undrawn_block_is_unread_and_scrolling_stops_at_the_last_window() {
        assert!(!AskView::default().seen(), "没画过的算没看完");
        let mut v = AskView {
            top: 0,
            rows: 10,
            room: 4,
        };
        assert_eq!((v.left(), v.seen()), (6, false));
        v.scroll(-3);
        assert_eq!(v.top, 0, "滚不出头");
        v.page(true);
        v.page(true);
        assert_eq!((v.top, v.seen()), (6, true), "到底了：最后一窗");
        v.scroll(5);
        assert_eq!(v.top, 6, "滚不出尾");
    }
}
