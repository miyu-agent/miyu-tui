//! 终端会话（设计 22 第七节，核心 `session.terminal`）：哪一个、什么时候换新的都由核心定，这里只问、只照它说的接上。

use serde_json::Value;

/// 问到的终端会话。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terminal {
    /// 会话编号。
    pub session: String,
    /// 闲置久了换了新会话：旧会话最后一次动静的时刻。
    pub renewed: Option<jiff::Timestamp>,
}

/// 读 `session.terminal` 的回应；读不懂的是 `None`（照没有终端会话，开新会话）。
pub fn read(result: &Value) -> Option<Terminal> {
    let session = result["session"].as_str()?.to_string();
    let renewed = result["renewed"]["last"]
        .as_str()
        .and_then(|t| t.parse::<jiff::Timestamp>().ok());
    Some(Terminal { session, renewed })
}

/// 隔了多久写成一个词：照分钟、小时、天，取最大的那一级（`3 小时`、`2 天`），不到一分钟的照一分钟。
pub fn ago(
    since: jiff::Timestamp,
    now: jiff::Timestamp,
    units: &crate::shellhook::words::Units,
) -> String {
    let minutes = now.duration_since(since).as_secs().max(60) / 60;
    let (n, unit) = if minutes >= 60 * 24 {
        (minutes / (60 * 24), &units.days)
    } else if minutes >= 60 {
        (minutes / 60, &units.hours)
    } else {
        (minutes, &units.minutes)
    };
    unit.replace("{n}", &n.to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Terminal, ago, read};

    #[test]
    fn a_renewed_terminal_session_says_when_the_old_one_was_last_used() {
        let at: jiff::Timestamp = "2026-10-20T01:00:00Z".parse().unwrap();
        assert_eq!(
            read(&json!({"session": "s", "renewed": {"last": "2026-10-20T01:00:00Z"}})),
            Some(Terminal {
                session: "s".into(),
                renewed: Some(at)
            })
        );
        assert_eq!(
            read(&json!({"session": "s", "created": true}))
                .unwrap()
                .renewed,
            None,
            "新开的不算换"
        );
        assert_eq!(read(&json!({})), None);
    }

    #[test]
    fn the_gap_is_written_in_its_largest_unit() {
        let units = crate::shellhook::words::Words::of("zh").unwrap().units;
        let at: jiff::Timestamp = "2026-10-20T00:00:00Z".parse().unwrap();
        let later = |s: i64| at.checked_add(jiff::SignedDuration::from_secs(s)).unwrap();
        assert_eq!(ago(at, later(10), &units), "1 分钟");
        assert_eq!(ago(at, later(45 * 60), &units), "45 分钟");
        assert_eq!(ago(at, later(5 * 3600 + 59), &units), "5 小时");
        assert_eq!(ago(at, later(3 * 86400), &units), "3 天");
    }
}
