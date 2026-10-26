//! 底栏、侧边栏的金额换算成一个币种（核心 UC-1，蓝图 `tui.md`「配置与模型」第 5 条）：核心只在订阅回应、`usage.query` 里给
//! 换算好的数，实时推来的 `model.called` 照旧是原币种。实时来的币种对得上的加进总数、对不上的先接在后面（`Converted::put`），
//! 一轮结束问一次这个会话的 `usage.query`，照核心换好的整份换掉，又是一个数。事件触发，不轮询。
//!
//! 一轮外又来一笔钱（起标题、回顾这类旁路请求在一轮结束以后才到）也再问一次。只在对得上的时候换：交回的原币种金额和
//! 这边一笔笔加的一样才换；问出去以后、回来以前又到了一笔的，对不上，照旧用这边的，等下一次再问。

use serde_json::json;

use super::App;
use crate::core::{Command, Push, Update};
use crate::oobe::asker::TAG_BASE;

/// 问这个会话合计的那一次。
const BILL_TAG: u64 = TAG_BASE - 21;

impl App {
    /// 正在看的会话一轮结束、一轮外又来了一笔钱（照会话分过以后的推送，`sessions.rs` 的 `route`：推来的都包着是哪个会话
    /// 的）：问一次。
    pub(super) fn bill_turn_ended(&mut self, update: &Update) {
        let ask = match update {
            Update::Push(Push::TurnEnded(_)) => true,
            Update::Push(Push::Billed(Some(_))) => self.talk.transcript.running.is_none(),
            _ => false,
        };
        if !ask {
            return;
        }
        if let Some(session) = self.talk.transcript.session.clone() {
            self.link.core.send(Command::Ask {
                system: false,
                tag: BILL_TAG,
                method: "usage.query",
                params: json!({ "session": session }),
            });
            self.link.bill_asked = Some(session);
        }
    }

    /// 问的回应归这里交回 `true`：照它换掉这个会话的账（问完又开了一轮的、换了会话的不换）。
    pub(super) fn bill_answer(&mut self, update: &Update) -> bool {
        match update {
            Update::Answer { tag, result } if *tag == BILL_TAG => {
                let asked = self.link.bill_asked.take();
                let same = asked.is_some() && asked == self.talk.transcript.session;
                if let Ok(result) = result
                    && same
                    && self.talk.transcript.running.is_none()
                    && let Some(row) = crate::core::usage_rows(result).into_iter().next()
                    && row.bill.same_amounts(&self.talk.transcript.bill)
                {
                    self.talk.transcript.bill = row.bill;
                }
                true
            }
            _ => false,
        }
    }
}
