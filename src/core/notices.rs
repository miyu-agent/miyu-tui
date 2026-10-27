//! 给人的通知（核心 J-3 中，`notices` 流；蓝图 `tui.md`「系统通知」第 8 条）：每次新开会话的定时跑完了、唤醒表坏了换了
//! 一份空的。连上就订（成员也能订，只收自己账号的；老核心不认的不说），掉了队重新订；不补以前的。

use serde_json::{Value, json};

/// 一条给人的通知。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// 每次新开会话的定时，开出来的那一轮结束了：`ok` 是答完了（出错、被打断的是假）。
    ScheduleRan { session: String, ok: bool },
    /// 唤醒表坏了，改名留着、换了一份空的：原来的定时都没了。
    SchedulesReset,
}

/// 订阅。交回请求编号。
pub(super) async fn follow(rpc: &mut super::rpc::Rpc) -> std::io::Result<String> {
    rpc.send("subscribe", json!({"stream": "notices"})).await
}

/// 推来的一条 `notice`；不是的、认不出的种类是 `None`。
pub fn read(message: &Value) -> Option<Notice> {
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
}
