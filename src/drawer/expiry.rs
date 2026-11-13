//! 确认的倒计时（核心 CD-1，todolist 第 49 条，2026-11-12 core1 转项目主人定：所有允许、拒绝的确认都倒计时，到时按拒绝、告诉
//! 她没人确认；她的提问不算）：`tool.approval_requested` 带 `expires_at` 的，抽屉问题下面暗色写「4:32 后自动拒绝」，一秒走一格
//! （同网页 W-1171）。到点核心记拒绝，抽屉照推来的决定收起。

use std::time::{Duration, SystemTime};

use super::{Ask, Drawer};

impl Drawer {
    /// 还剩多久自动拒绝：不是确认、没带 `expires_at`、读不懂的是 `None`；过了的是 0。
    pub fn expires_in(&self, now: SystemTime) -> Option<Duration> {
        let Ask::Approval(a) = &self.ask else {
            return None;
        };
        let at: jiff::Timestamp = a.expires_at.as_deref()?.parse().ok()?;
        let at = SystemTime::from(at);
        Some(at.duration_since(now).unwrap_or_default())
    }
}

/// 剩下的时间写成 `4:32`，一小时以上 `1:04:32`；不满一秒的往上取（剩 0.2 秒写 `0:01`，到 0 才写 `0:00`）。
pub fn clock(left: Duration) -> String {
    let secs = left.as_millis().div_ceil(1000);
    let (h, m, s) = (secs / 3600, secs % 3600 / 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// 下一次要醒来重画的那一格：剩下的时间跳到下一个整秒的时候（没剩的不用醒）。
pub fn next_tick(left: Duration) -> Option<Duration> {
    if left.is_zero() {
        return None;
    }
    let ms = left.as_millis() % 1000;
    Some(Duration::from_millis(if ms == 0 {
        1000
    } else {
        ms as u64
    }))
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::{clock, next_tick};
    use crate::drawer::Drawer;

    #[test]
    fn the_clock_rounds_up_and_shows_hours_past_one() {
        assert_eq!(clock(Duration::from_secs(300)), "5:00");
        assert_eq!(clock(Duration::from_millis(1200)), "0:02");
        assert_eq!(clock(Duration::from_secs(3872)), "1:04:32");
        assert_eq!(clock(Duration::ZERO), "0:00");
        assert_eq!(
            next_tick(Duration::from_millis(4300)),
            Some(Duration::from_millis(300))
        );
        assert_eq!(next_tick(Duration::ZERO), None);
    }

    #[test]
    fn only_an_approval_with_expires_at_counts_down() {
        let body = |extra: serde_json::Value| {
            let mut b = serde_json::json!({"call_id": "c1", "access": "write"});
            b.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            Drawer::approval(None, serde_json::from_value(b).unwrap())
        };
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let at = jiff::Timestamp::from_second(1_800_000_272)
            .unwrap()
            .to_string();
        let timed = body(serde_json::json!({"expires_at": at}));
        assert_eq!(timed.expires_in(now), Some(Duration::from_secs(272)));
        assert_eq!(
            body(serde_json::json!({})).expires_in(now),
            None,
            "timeout 是 0 的不带"
        );
        assert_eq!(
            timed.expires_in(now + Duration::from_secs(400)),
            Some(Duration::ZERO)
        );
    }
}
