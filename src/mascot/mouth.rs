//! 吉祥物的嘴（蓝图 `tui.md`「空会话的首页」第 9 条，照网页演示）：锯齿线是嘴，张多大是 0 合着到 1 张到最大，照半衰期
//! 缓着变。被列表顶上去张一下、走下来时一路张着；醒着的那一阵过了打一个哈欠（张到最大、闭着眼），打完停住。

use std::time::{Duration, Instant};

use super::model::MouthLook as Look;

/// 嘴：现在张多大，一会儿要张成多大、到什么时候、缓多快，什么时候打哈欠。
#[derive(Debug, Default)]
pub struct Mouth {
    now: f64,
    /// 张一阵：张成多大、到什么时候、缓动的半衰期。
    gaping: Option<(f64, Instant, Duration)>,
    /// 这一刻起一路张着（走下来）：张多大；没有是 `None`。
    holding: Option<f64>,
    /// 正在打哈欠：到这一刻为止（闭着眼）。
    yawn_until: Option<Instant>,
    /// 到这一刻打哈欠（醒着的那一阵到头的时刻）；打过了是 `None`。
    next_yawn: Option<Instant>,
    /// 为哪一刻打过哈欠：同一阵只打一次，有了动静、那一阵重新算才再打。
    yawned: Option<Instant>,
    at: Option<Instant>,
}

impl Mouth {
    /// 合着的。
    pub fn new() -> Self {
        Self::default()
    }

    /// 张成 `level` 张 `ms` 毫秒再回去，照 `half_ms` 缓着变。
    pub fn gape(&mut self, now: Instant, level: f64, ms: u64, half_ms: u64) {
        let until = now + Duration::from_millis(ms);
        self.gaping = Some((level, until, Duration::from_millis(half_ms)));
    }

    /// 被列表顶上去：张一下。
    pub fn hop(&mut self, now: Instant, look: &Look) {
        self.gape(now, look.hop, look.hop_ms, look.half_life_ms);
    }

    /// 一行一行走下来时一路张着；走完了给 `None`。
    pub fn hold(&mut self, level: Option<f64>) {
        self.holding = level;
    }

    /// 走到 `now` 这一刻，交回张多大。`rest_at` 是醒着的那一阵到头的时刻（`Idle::awake_until`）：到了打一个哈欠，
    /// 同一阵只打一次。
    pub fn step(&mut self, now: Instant, look: &Look, rest_at: Option<Instant>) -> f64 {
        self.next_yawn = rest_at.filter(|at| self.yawned != Some(*at));
        if let Some(at) = self.next_yawn.filter(|at| now >= *at) {
            self.yawned = Some(at);
            self.next_yawn = None;
            self.gape(now, look.yawn, look.yawn_ms, look.yawn_half_life_ms);
            self.yawn_until = Some(now + Duration::from_millis(look.yawn_ms));
        }
        if self.gaping.is_some_and(|(_, until, _)| now >= until) {
            self.gaping = None;
        }
        let (want, half) = match (self.gaping, self.holding) {
            (Some((level, _, half)), _) => (level, half),
            (None, Some(level)) => (level, Duration::from_millis(look.half_life_ms)),
            (None, None) => (look.rest, Duration::from_millis(look.half_life_ms)),
        };
        let dt = self
            .at
            .map_or(Duration::ZERO, |at| now.saturating_duration_since(at))
            .min(half.max(Duration::from_millis(1)));
        self.at = Some(now);
        let keep = if half.is_zero() {
            0.0
        } else {
            0.5_f64.powf(dt.as_secs_f64() / half.as_secs_f64())
        };
        let next = want + (self.now - want) * keep;
        self.now = if (next - want).abs() < 0.02 {
            want
        } else {
            next
        };
        self.now
    }

    /// 在打哈欠：闭着眼。
    pub fn yawning(&self, now: Instant) -> bool {
        self.yawn_until.is_some_and(|u| now < u)
    }

    /// 下一次该画的时刻：张着、变着的时候照 `frame_ms` 一帧；停着的等打哈欠的那一刻，打过了不再醒来。
    pub fn wake(&self, now: Instant, look: &Look) -> Option<Instant> {
        let moving = self.gaping.is_some()
            || (self.now - self.holding.unwrap_or(look.rest)).abs() > f64::EPSILON;
        if moving {
            return Some(now + Duration::from_millis(look.frame_ms));
        }
        self.next_yawn.filter(|at| *at > now)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::Mouth;
    use crate::config::Config;

    #[test]
    fn a_hop_opens_then_closes_again() {
        let look = Config::builtin().unwrap().mascot.mouth;
        let t0 = Instant::now();
        let mut m = Mouth::new();
        assert_eq!(m.step(t0, &look, None), look.rest, "平常合着");
        m.hop(t0, &look);
        let mut open = 0.0_f64;
        for k in 1..=5 {
            open = open.max(m.step(t0 + Duration::from_millis(16 * k), &look, None));
        }
        assert!(open > 0.3, "被顶上去张开了：{open}");
        let later = t0 + Duration::from_millis(look.hop_ms + 400);
        for k in 0..30 {
            m.step(later + Duration::from_millis(16 * k), &look, None);
        }
        assert_eq!(
            m.step(later + Duration::from_millis(600), &look, None),
            look.rest,
            "张完合上"
        );
        assert_eq!(
            m.wake(later + Duration::from_millis(600), &look),
            None,
            "合上了、没有哈欠要打：不再醒来"
        );
    }

    #[test]
    fn it_yawns_once_when_the_awake_spell_ends_and_closes_the_eyes() {
        let look = Config::builtin().unwrap().mascot.mouth;
        let t0 = Instant::now();
        let rest = t0 + Duration::from_secs(10);
        let mut m = Mouth::new();
        m.step(t0, &look, Some(rest));
        assert!(!m.yawning(t0), "醒着不打");
        assert_eq!(m.wake(t0, &look), Some(rest), "停着只等那一刻");
        m.step(rest, &look, Some(rest));
        assert!(m.yawning(rest), "到点闭着眼打哈欠");
        let mut open = 0.0_f64;
        let mut t = rest;
        for _ in 1..=200 {
            t += Duration::from_millis(16);
            open = open.max(m.step(t, &look, Some(rest)));
        }
        assert!(open > 0.9, "张到最大：{open}");
        assert!(!m.yawning(t), "打完睁眼");
        assert_eq!(m.step(t, &look, Some(rest)), look.rest, "打完合上");
        assert_eq!(m.wake(t, &look), None, "同一阵只打一次，打完不再醒来");
        // 有了动静、那一阵重新算：到头再打一次。
        let again = t + Duration::from_secs(10);
        m.step(again, &look, Some(again));
        assert!(m.yawning(again));
    }

    #[test]
    fn walking_down_keeps_it_open() {
        let look = Config::builtin().unwrap().mascot.mouth;
        let t0 = Instant::now();
        let mut m = Mouth::new();
        m.hold(Some(look.walk));
        for k in 0..40 {
            m.step(t0 + Duration::from_millis(16 * k), &look, None);
        }
        assert!((m.step(t0 + Duration::from_millis(700), &look, None) - look.walk).abs() < 0.03);
        m.hold(None);
        for k in 0..40 {
            m.step(t0 + Duration::from_millis(700 + 16 * k), &look, None);
        }
        assert_eq!(
            m.step(t0 + Duration::from_millis(1400), &look, None),
            look.rest
        );
    }
}
