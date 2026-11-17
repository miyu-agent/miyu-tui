//! 人生会话的列表（核心 LF-12、LF-14，`docs/reference/life.md` 的 `life.list`）：所有人格没结束的人生会话，光打 `/life` 的回应
//! 里带同一份（这个会话的人格那一条排最前）；`life.list` 另带她自己结束了、还没人看过的那几段（`ended_unseen`）。

use jiff::Timestamp;
use serde_json::Value;

/// 一条没结束的人生会话。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Life {
    /// 人生会话的编号。
    pub session: String,
    /// 人格的编号。
    pub persona: String,
    /// 人格给人看的名字（核心照连接的语言挑好，没起名字的是编号）。
    pub name: String,
    /// 暂停着（`state` 是 `paused`）。
    pub paused: bool,
    /// 下一轮开始的时刻；只有在跑的有。
    pub next: Option<Timestamp>,
    /// 这会儿正在跑一轮（核心 WK-2 起叫 `busy`）。
    pub busy: bool,
}

/// 她自己结束了、还没人看过的一段人生（`ended_unseen` 的一项）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ended {
    /// 那个人生会话。
    pub session: String,
    /// 人格的名字；会话那时不带人格的是 `None`。
    pub name: Option<String>,
}

/// 读 `lives`（`life.list` 的回应、光打 `/life` 的回应里那一格）；没有这一格的是空的。
pub fn lives(got: &Value) -> Vec<Life> {
    let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
    got.as_array()
        .into_iter()
        .flatten()
        .filter_map(|l| {
            let session = l["session"].as_str()?.to_string();
            let persona = text(&l["persona"]);
            let name = l["name"]
                .as_str()
                .map_or_else(|| persona.clone(), str::to_string);
            Some(Life {
                session,
                persona,
                name,
                paused: l["state"] == "paused",
                next: l["next"].as_str().and_then(|t| t.parse().ok()),
                busy: l["busy"] == true,
            })
        })
        .collect()
}

/// 读 `ended_unseen`：结束得早的在前，照核心的先后。
pub fn ended(got: &Value) -> Vec<Ended> {
    got.as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| {
            Some(Ended {
                session: e["session"].as_str()?.to_string(),
                name: e["name"].as_str().map(str::to_string),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Ended, ended, lives};

    #[test]
    fn lives_and_unseen_ends_are_read_as_the_core_sends_them() {
        let got = lives(&json!([
            {"state": "running", "session": "s-1", "next": "2026-11-14T10:00:00.000Z", "persona": "miyu", "name": "美羽"},
            {"state": "paused", "session": "s-2", "persona": "luna", "busy": true},
            {"state": "running", "persona": "没有会话的不要"}
        ]));
        assert_eq!(got.len(), 2);
        assert_eq!(
            (got[0].name.as_str(), got[0].paused, got[0].next.is_some()),
            ("美羽", false, true)
        );
        assert_eq!(
            (got[1].name.as_str(), got[1].paused, got[1].busy),
            ("luna", true, true),
            "没起名字的写编号"
        );
        assert!(lives(&json!(null)).is_empty());
        assert_eq!(
            ended(
                &json!([{"session": "s-9", "persona": "miyu", "name": "美羽", "at": "2026-11-17T08:00:00.000Z"}, {"session": "s-8", "at": "x"}])
            ),
            [
                Ended {
                    session: "s-9".into(),
                    name: Some("美羽".into())
                },
                Ended {
                    session: "s-8".into(),
                    name: None
                }
            ]
        );
    }
}
