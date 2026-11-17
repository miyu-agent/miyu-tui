//! 给人的通知（核心 J-3 中，`notices` 流；蓝图 `tui.md`「系统通知」第 8 条）：每次新开会话的定时跑完了、唤醒表坏了换了
//! 一份空的，一段人生结束了、被看过了（核心 LF-14）。连上就订（老核心不认的不说），掉了队重新订；不补以前的。

use serde_json::{Value, json};

/// 一条给人的通知。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// 每次新开会话的定时，开出来的那一轮结束了：`ok` 是答完了（出错、被打断的是假）。
    ScheduleRan { session: String, ok: bool },
    /// 唤醒表坏了，改名留着、换了一份空的：原来的定时都没了。
    SchedulesReset,
    /// 有没有人认领了后台通知（核心 TR-1：订阅回应的 `claimed`、推来的 `notices.claimed`）。
    Claimed(bool),
    /// 一段人生结束了（核心 LF-14）：那个人生会话、人格给人看的名字（不带人格的是 `None`）、是不是她自己结束的（`by` 是
    /// `self`；人结束的是 `owner`）。遗言不读：界面不显示（主人定）。
    LifeEnded {
        session: String,
        name: Option<String>,
        by_self: bool,
    },
    /// 一段她自己结束了、还没看过的人生被看过了（有头打开了那个会话、调了 `life.seen`、会话删了）：收起提示。
    LifeSeen { session: String },
}

/// 订阅。交回请求编号。
pub(super) async fn follow(rpc: &mut super::rpc::Rpc) -> std::io::Result<String> {
    rpc.send("subscribe", json!({"stream": "notices"})).await
}

/// 订阅的回应：这一刻有人认领了的写 `{"claimed": true}`，没有的、旧核心是 `{}`（核心 TR-1）。
pub fn subscribed(result: &Value) -> Notice {
    Notice::Claimed(result["claimed"] == true)
}

/// 推来的一条 `notice`、认领变了的 `notices.claimed`；不是的、认不出的种类是 `None`。
pub fn read(message: &Value) -> Option<Notice> {
    if message["method"] == "notices.claimed" {
        return Some(Notice::Claimed(message["params"]["claimed"] == true));
    }
    if message["method"] != "notice" {
        return None;
    }
    let params = &message["params"];
    match params["kind"].as_str()? {
        "schedule.ran" => Some(Notice::ScheduleRan {
            session: params["session"].as_str()?.to_string(),
            ok: params["ok"].as_bool().unwrap_or(false),
        }),
        "schedules.reset" => Some(Notice::SchedulesReset),
        "life.ended" => Some(Notice::LifeEnded {
            session: params["session"].as_str()?.to_string(),
            name: params["name"].as_str().map(str::to_string),
            by_self: params["by"] == "self",
        }),
        "life.seen" => Some(Notice::LifeSeen {
            session: params["session"].as_str()?.to_string(),
        }),
        _ => None,
    }
}

/// 掉了队：要重新订。
pub fn lost(message: &Value) -> bool {
    message["method"] == "resync" && message["params"]["stream"] == "notices"
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Notice, lost, read};

    #[test]
    fn the_two_kinds_of_notice_are_read_and_the_rest_ignored() {
        let push = |params| json!({"jsonrpc": "2.0", "method": "notice", "params": params});
        assert_eq!(
            read(&push(
                json!({"kind": "schedule.ran", "at": "2026-10-27T08:00:00.000Z",
                "schedule": "t3", "session": "s-1", "ok": true})
            )),
            Some(Notice::ScheduleRan {
                session: "s-1".into(),
                ok: true
            })
        );
        assert_eq!(
            read(&push(
                json!({"kind": "schedule.ran", "schedule": "t3", "session": "s-1", "ok": false})
            )),
            Some(Notice::ScheduleRan {
                session: "s-1".into(),
                ok: false
            })
        );
        assert_eq!(
            read(&push(
                json!({"kind": "schedules.reset", "at": "2026-10-27T08:00:00.000Z"})
            )),
            Some(Notice::SchedulesReset)
        );
        assert_eq!(read(&push(json!({"kind": "以后才有的"}))), None);
        assert!(lost(
            &json!({"method": "resync", "params": {"stream": "notices"}})
        ));
        assert!(!lost(
            &json!({"method": "resync", "params": {"stream": "sessions"}})
        ));
    }

    /// 核心 LF-14：一段人生结束了（带名字，`by` 分她自己、人）、被看过了；遗言不读。
    #[test]
    fn a_life_that_ended_and_was_seen_is_read() {
        let push = |params| json!({"jsonrpc": "2.0", "method": "notice", "params": params});
        assert_eq!(
            read(&push(
                json!({"kind": "life.ended", "at": "2026-11-17T08:00:00.000Z", "persona": "miyu",
                "name": "美羽", "session": "s-life", "by": "self", "last_words": "再见"})
            )),
            Some(Notice::LifeEnded {
                session: "s-life".into(),
                name: Some("美羽".into()),
                by_self: true
            })
        );
        assert_eq!(
            read(&push(
                json!({"kind": "life.ended", "session": "s-2", "by": "owner"})
            )),
            Some(Notice::LifeEnded {
                session: "s-2".into(),
                name: None,
                by_self: false
            })
        );
        assert_eq!(
            read(&push(json!({"kind": "life.seen", "session": "s-life"}))),
            Some(Notice::LifeSeen {
                session: "s-life".into()
            })
        );
    }

    /// 核心 TR-1：订阅回应的 `claimed`、推来的 `notices.claimed`。
    #[test]
    fn claims_come_from_the_subscription_and_a_push() {
        use super::subscribed;
        assert_eq!(subscribed(&json!({"claimed": true})), Notice::Claimed(true));
        assert_eq!(
            subscribed(&json!({})),
            Notice::Claimed(false),
            "没人认领、旧核心"
        );
        let push = json!({"method": "notices.claimed", "params": {"claimed": true}});
        assert_eq!(read(&push), Some(Notice::Claimed(true)));
    }
}
