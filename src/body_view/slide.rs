//! 抽屉跟着正文滚（蓝图 `tui.md`「确认和提问的抽屉」第 4a 条，2026-10-27 项目主人：「Claude Code 是可以做到让确认框
//! 无级滚动的」，原来翻离底部抽屉一下子让开、滚一格就消失再出现）：正文在最底下、开着抽屉时，往上滚先让抽屉一行行往下
//! 滑（滑出去的那一截被框下面那一行挡住），正文跟着往下露出一样多的行；整个滑出去以后再往上才翻正文。往下滚反过来：
//! 正文先回到最底下，再把抽屉一行行滑回来。

use super::BodyView;

/// 抽屉往下滑了几行。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Slide {
    /// 抽屉连上下两条边一共几行；没开抽屉是 0（每一帧画的时候填，[`BodyView::drawer`]）。
    pub rows: u16,
    /// 往下滑了几行：0 在原位，到 `rows` 整个滑出去。
    pub by: u16,
}

impl Slide {
    /// 整个滑出去了：抽屉不画，正文用满它的位置。
    pub fn away(self) -> bool {
        self.rows > 0 && self.by >= self.rows
    }
}

impl BodyView {
    /// 这一帧抽屉连边框几行（没开的是 0）：没开了就回原位，变矮了滑的行数跟着收。
    pub fn drawer(&mut self, rows: u16) {
        self.slide.rows = rows;
        self.slide.by = if rows == 0 {
            0
        } else {
            self.slide.by.min(rows)
        };
    }

    /// 往上（`up`）、往下滚 `n` 行（滚轮、`PgUp` `PgDn`）。
    pub(super) fn scroll_by(&mut self, up: bool, n: usize) {
        let following = self.top.is_none();
        let n16 = u16::try_from(n).unwrap_or(u16::MAX);
        if up {
            let room = self.slide.rows.saturating_sub(self.slide.by);
            let take = if following { n16.min(room) } else { 0 };
            self.slide.by += take;
            let rest = n.saturating_sub(usize::from(take));
            if rest > 0 {
                self.scroll_to(self.first.saturating_sub(rest));
            }
        } else if following {
            // 已经在最底下：只把滑下去的抽屉滑回来，不定第一行（2026-10-27 项目主人报：在底下往下滚，抽屉闪一下）。
            self.slide.by = self.slide.by.saturating_sub(n16);
        } else {
            self.scroll_to(self.first + n);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::BodyView;

    #[test]
    fn scrolling_up_slides_the_drawer_away_first_and_down_brings_it_back() {
        let mut v = BodyView {
            first: 40,
            ..BodyView::default()
        };
        v.drawer(8);
        v.scroll_by(true, 3);
        assert_eq!((v.slide.by, v.top), (3, None), "先滑抽屉，正文还跟着最新的");
        v.scroll_by(true, 3);
        v.scroll_by(true, 3);
        assert_eq!(v.slide.by, 8, "滑到整个出去为止");
        assert_eq!(v.top, Some(39), "多出来的一行才翻正文");
        assert!(v.slide.away());
        v.top = None;
        v.scroll_by(false, 3);
        assert_eq!(
            (v.slide.by, v.top),
            (5, None),
            "在最底下往下滚：抽屉滑回来，不定第一行"
        );
        v.scroll_by(false, 30);
        assert_eq!((v.slide.by, v.top), (0, None));
        v.scroll_by(false, 3);
        assert_eq!(v.top, None, "已经在底下、抽屉在原位：再往下滚什么都不动");
        v.drawer(0);
        v.scroll_by(true, 3);
        assert_eq!((v.slide.by, v.top), (0, Some(37)), "没开抽屉：直接翻正文");
    }
}
