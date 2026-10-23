//! 正文的滚动条（蓝图「正文」第 1a 条，2026-10-22 项目主人照 Pi 要的）：只管算——滑块在轨道的哪一段（带小数，画成八分之
//! 一格，2026-10-23 拖动改按像素）、拖到哪该滚到第几行。画在 `ui/body.rs`，鼠标在 `body_view.rs`。

/// 内容放不下时的一根滚动条：轨道高 `height` 格，滑块从第 `start` 格起、`len` 格长（都带小数）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    pub start: f64,
    pub len: f64,
    height: u16,
    /// 滚到底时第一行是第几行。
    last_first: usize,
}

impl Bar {
    /// 一共 `total` 行、露 `height` 行、第一行是第 `first` 行；放得下的没有滚动条。
    pub fn new(total: usize, height: u16, first: usize) -> Option<Self> {
        let h = usize::from(height);
        if h == 0 || total <= h {
            return None;
        }
        let hf = f64::from(height);
        let len = (hf * hf / total as f64).clamp(1.0, hf);
        let last_first = total - h;
        let start = first.min(last_first) as f64 / last_first as f64 * (hf - len);
        Some(Self {
            start,
            len,
            height,
            last_first,
        })
    }

    /// 轨道第 `y` 格在不在滑块上（按住哪儿算抓着滑块）。
    pub fn holds(&self, y: u16) -> bool {
        let y = f64::from(y);
        y + 1.0 > self.start && y < self.start + self.len
    }

    /// 第 `y` 格被滑块盖住多少（0 到 1）、滑块是从这一格当中开始的（盖住的是下半截）还是在这一格当中结束的（上半截）。
    pub fn cover(&self, y: u16) -> (f64, bool) {
        let (top, bottom) = (f64::from(y), f64::from(y) + 1.0);
        let from = self.start.max(top);
        let to = (self.start + self.len).min(bottom);
        ((to - from).max(0.0), self.start > top)
    }

    /// 抓着滑块往下第 `grab` 格（带小数）、拖到轨道第 `y` 格（带小数）时，第一行该是第几行。
    pub fn first_at(&self, y: f64, grab: f64) -> usize {
        let room = f64::from(self.height) - self.len;
        if room <= 0.0 {
            return 0;
        }
        let share = ((y - grab) / room).clamp(0.0, 1.0);
        (share * self.last_first as f64).round() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::Bar;

    #[test]
    fn the_thumb_shows_how_much_is_seen_and_where() {
        assert_eq!(Bar::new(20, 20, 0), None, "放得下的不画");
        let top = Bar::new(100, 20, 0).expect("有");
        assert_eq!(
            (top.start, top.len),
            (0.0, 4.0),
            "露五分之一：滑块占五分之一"
        );
        let bottom = Bar::new(100, 20, 80).expect("有");
        assert_eq!(bottom.start, 16.0, "滚到底：滑块贴底");
        assert_eq!(Bar::new(100, 20, 40).expect("有").start, 8.0, "滚到一半");
        assert_eq!(
            Bar::new(100_000, 20, 0).expect("有").len,
            1.0,
            "再长也至少一格"
        );
        // 滚到不是整格的地方：滑块照小数，头尾两格各盖一部分。
        let mid = Bar::new(100, 20, 10).expect("有");
        assert_eq!(mid.start, 2.0);
        let half = Bar::new(100, 20, 5).expect("有");
        assert_eq!(half.cover(0).0, 0.0, "滑块从第 1 格起");
        assert_eq!(half.cover(1), (1.0, false));
        let (c, starts) = Bar::new(160, 20, 7).expect("有").cover(0);
        assert!(c > 0.0 && c < 1.0 && starts, "从第 0 格当中开始：{c}");
    }

    #[test]
    fn dragging_or_clicking_maps_back_to_a_row() {
        let bar = Bar::new(100, 20, 0).expect("有");
        assert!(bar.holds(0) && bar.holds(3) && !bar.holds(4));
        assert_eq!(bar.first_at(16.0, 0.0), 80, "拖到底");
        assert_eq!(bar.first_at(10.0, 2.0), 40, "抓着滑块第 2 格拖到第 10 格");
        assert_eq!(bar.first_at(0.0, 2.0), 0, "拖出顶边算顶");
        assert_eq!(bar.first_at(19.0, 0.0), 80, "拖出底边算底");
        // 按像素拖：挪半格就滚一半格那么多行（这里一格五行，半格两三行）。
        assert_eq!(bar.first_at(0.5, 0.0), 3);
    }
}
