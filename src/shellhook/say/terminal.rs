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

/// 这一句、这个斜杠命令问终端会话问得怎样（`Turn::terminal`）。每个终端各一个终端会话（核心 T-22，2026-10-23 项目主人：
/// 「不同终端不同会话……只有真的对话了才开新会话，不是开了终端就开新会话」）。
#[derive(Debug, Default)]
pub struct Asked {
    /// 这个终端的键（[`key`]）；没有的（不是从 shell 的脚本调的）照账号原来那一个指针。
    pub key: Option<String>,
    /// 只问不开（`create: false`）：斜杠命令这些不说话的，这个终端还没说过话就不开。
    pub only_ask: bool,
    /// 只问不开、这个终端还没有终端会话（核心回 `session: null`）。
    pub none: bool,
    /// 闲置久了核心换了新会话：旧会话最后一次动静的时刻，印一行就拿走。
    pub renewed: Option<jiff::Timestamp>,
}

impl Asked {
    /// `session.terminal` 的参数：新开的开在 `cwd`（shell 所在的目录）。
    pub fn params(&self, cwd: Option<&str>) -> Value {
        let mut params = serde_json::json!({ "cwd": cwd });
        if let Some(key) = &self.key {
            params["terminal"] = Value::from(key.as_str());
        }
        if self.only_ask {
            params["create"] = Value::Bool(false);
        }
        params
    }

    /// 核心拒了带键、只问不开的问法（T-22 以前的核心不认这两格）：照原来的问法再问一次，交回要不要再问。
    pub fn again<T>(&mut self, result: &Result<Value, T>) -> bool {
        let new = self.key.is_some() || self.only_ask;
        if result.is_ok() || !new {
            return false;
        }
        self.key = None;
        self.only_ask = false;
        true
    }

    /// 读回应：接上的那一个；只问不开、还没有的记下 `none`。
    pub fn answered(&mut self, result: Option<&Value>) -> Option<Terminal> {
        let found = result.and_then(read);
        self.none = self.only_ask && found.is_none();
        self.renewed = found.as_ref().and_then(|f| f.renewed);
        found
    }
}

/// 这个终端的键：照 shell 的进程号和它起来的时刻（Linux 读 `/proc/<pid>/stat` 第 22 项，进程号回收了也认得出不是同一个），
/// 别的平台只照进程号。不是从 shell 的脚本调的（没有进程号）是 `None`。
pub fn key(shell: Option<u32>) -> Option<String> {
    let pid = shell?;
    Some(match started(pid) {
        Some(at) => format!("shell-{pid}-{at}"),
        None => format!("shell-{pid}"),
    })
}

/// 进程起来的时刻（开机以来的时钟数）；读不到的是 `None`。
fn started(pid: u32) -> Option<u64> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // 第 2 项是括起来的程序名，里面可能有空格：从最后一个右括号往后数。
    let rest = &stat[stat.rfind(')')? + 1..];
    rest.split_whitespace().nth(19)?.parse().ok()
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

    use super::{Asked, Terminal, ago, key, read};

    #[test]
    fn only_asking_sends_create_false_and_a_null_answer_means_none() {
        let mut asked = Asked {
            key: Some("shell-7".into()),
            only_ask: true,
            ..Asked::default()
        };
        assert_eq!(
            asked.params(Some("/w")),
            json!({"cwd": "/w", "terminal": "shell-7", "create": false})
        );
        assert!(asked.answered(Some(&json!({"session": null}))).is_none());
        assert!(asked.again::<()>(&Err(())), "旧核心不认：照原来的问法再问");
        assert_eq!(asked.params(None), json!({"cwd": null}));
        assert!(!asked.again::<()>(&Err(())), "原来的问法也拒了：不再问");
        assert!(asked.none, "这个终端还没说过话");
        let talk = Asked::default();
        assert_eq!(
            talk.params(None),
            json!({"cwd": null}),
            "说话的照旧：不带键的是原来那一个指针"
        );
    }

    #[test]
    fn a_terminal_is_its_shell_and_when_it_started() {
        assert_eq!(key(None), None);
        let me = std::process::id();
        let mine = key(Some(me)).unwrap();
        assert!(mine.starts_with(&format!("shell-{me}")));
        assert_eq!(key(Some(me)), Some(mine), "同一个 shell 每次一样");
        if cfg!(target_os = "linux") {
            assert!(
                key(Some(me)).unwrap().matches('-').count() == 2,
                "带上起来的时刻"
            );
        }
    }

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
