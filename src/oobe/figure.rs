//! 引导左边那一块画什么（蓝图 `tui.md`「第一次打开的引导」第 11a 条，2026-10-15 项目主人）：吉祥物、人格的头像，或者什么都
//! 不画（主题那一步选了吉祥物不显示）。要画的一变就当场换，带动画：吉祥物往左走出画面，头像在原位从上往下一行行露出来；
//! 换回吉祥物的从左边走回来。头像不跟着换步跳（项目主人：不要给头像弹跳动画）。

use std::time::{Duration, Instant};

use serde::Deserialize;

/// 画什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shown {
    #[default]
    Mascot,
    Avatar,
    Nothing,
}

/// 换的时候走多久、露多久（`oobe.json` 的 `swap`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Swap {
    /// 吉祥物走出去、走回来多久。
    pub walk_ms: u64,
    /// 头像从上往下露出来多久。
    pub reveal_ms: u64,
}

/// 这一刻画成什么样。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Phase {
    /// 画吉祥物：往左走出去了几成（0 在原处，1 整个走出画面）。
    pub mascot: Option<f64>,
    /// 画头像：露出来几成（1 整张）。
    pub avatar: Option<f64>,
}

/// 左边那一块：现在要画的、从哪种换过来、什么时候起换（还没换过的照现在要画的直接画）。
#[derive(Debug, Clone, Default)]
pub struct Figure {
    pub shown: Shown,
    from: Shown,
    since: Option<Instant>,
}

impl Figure {
    /// 这一帧要画 `wanted`：和现在的不一样的，从这一刻起换。
    pub fn want(&mut self, wanted: Shown, now: Instant) {
        if wanted != self.shown {
            self.from = self.shown;
            self.shown = wanted;
            self.since = Some(now);
        }
    }

    /// 还在换：要接着画帧。
    pub fn moving(&self, now: Instant, look: &Swap) -> bool {
        self.since.is_some_and(|since| {
            now.saturating_duration_since(since)
                < Duration::from_millis(look.walk_ms + look.reveal_ms)
        })
    }

    /// 这一刻画成什么样：吉祥物要走开的先走出去，再露头像；换回吉祥物的头像当场收掉，吉祥物从左边走回来。
    pub fn phase(&self, now: Instant, look: &Swap) -> Phase {
        let t = self.since.map_or(f64::INFINITY, |s| {
            now.saturating_duration_since(s).as_millis() as f64
        });
        let walk = look.walk_ms.max(1) as f64;
        let reveal = look.reveal_ms.max(1) as f64;
        let leaving = self.from == Shown::Mascot && self.shown != Shown::Mascot;
        // 吉祥物还在走出去：头像等它走完再露。
        if leaving && t < walk {
            return Phase {
                mascot: Some(t / walk),
                avatar: None,
            };
        }
        match self.shown {
            Shown::Mascot => {
                let back = self.from != Shown::Mascot && t < walk;
                Phase {
                    mascot: Some(if back { 1.0 - t / walk } else { 0.0 }),
                    avatar: None,
                }
            }
            Shown::Avatar => {
                let start = if leaving { walk } else { 0.0 };
                Phase {
                    mascot: None,
                    avatar: Some(((t - start) / reveal).clamp(0.0, 1.0)),
                }
            }
            Shown::Nothing => Phase {
                mascot: None,
                avatar: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{Figure, Phase, Shown, Swap};

    const LOOK: Swap = Swap {
        walk_ms: 600,
        reveal_ms: 900,
    };

    fn at(figure: &Figure, t0: Instant, ms: u64) -> Phase {
        figure.phase(t0 + Duration::from_millis(ms), &LOOK)
    }

    #[test]
    fn the_mascot_walks_out_then_the_avatar_is_revealed_in_its_place() {
        let t0 = Instant::now();
        let mut figure = Figure::default();
        assert_eq!(
            at(&figure, t0, 0).mascot,
            Some(0.0),
            "还没换过：吉祥物在原处"
        );
        figure.want(Shown::Avatar, t0);
        assert_eq!(
            at(&figure, t0, 300),
            Phase {
                mascot: Some(0.5),
                avatar: None
            }
        );
        assert_eq!(
            at(&figure, t0, 600 + 450),
            Phase {
                mascot: None,
                avatar: Some(0.5)
            }
        );
        assert_eq!(at(&figure, t0, 5000).avatar, Some(1.0));
        assert!(figure.moving(t0 + Duration::from_millis(1000), &LOOK));
        assert!(!figure.moving(t0 + Duration::from_millis(1600), &LOOK));
    }

    #[test]
    fn turning_the_mascot_off_walks_it_out_and_on_again_walks_it_back() {
        let t0 = Instant::now();
        let mut figure = Figure::default();
        figure.want(Shown::Nothing, t0);
        assert_eq!(at(&figure, t0, 300).mascot, Some(0.5));
        assert_eq!(
            at(&figure, t0, 700),
            Phase {
                mascot: None,
                avatar: None
            }
        );
        let t1 = t0 + Duration::from_millis(2000);
        figure.want(Shown::Mascot, t1);
        assert_eq!(
            figure.phase(t1 + Duration::from_millis(150), &LOOK).mascot,
            Some(0.75)
        );
        assert_eq!(
            figure.phase(t1 + Duration::from_millis(900), &LOOK).mascot,
            Some(0.0)
        );
    }

    #[test]
    fn the_same_figure_does_not_move() {
        let t0 = Instant::now();
        let mut figure = Figure::default();
        figure.want(Shown::Mascot, t0);
        assert!(!figure.moving(t0, &LOOK), "一样的不换");
        assert_eq!(
            at(&figure, t0, 0),
            Phase {
                mascot: Some(0.0),
                avatar: None
            }
        );
    }
}
