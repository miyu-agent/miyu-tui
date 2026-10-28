//! 底栏、侧边栏的金额换算成一个币种（核心 UC-1，蓝图 `tui.md`「配置与模型」第 5 条）：核心只在订阅回应、`usage.query` 里给
//! 换算好的数，实时推来的 `model.called` 照旧是原币种。实时来的币种对得上的加进总数、对不上的先接在后面（`Converted::put`），
//! 一轮结束问一次这个会话连同它派的子会话的 `usage.query`（`tree`），照核心换好的整份写一个数（`Talk::bill_total`）。
//! 事件触发，不轮询。
//!
//! 写的总数连同子代理花的（2026-10-27 自测发现：原来只问主会话那一份，派过子代理以后一直是「$… + ¥…」），所以子代理一轮
//! 结束、一轮外又来一笔钱也再问一次。只在对得上的时候用：交回的原币种金额和这边一笔笔加的一样才用；问出去以后、回来以前
//! 又到了一笔的，对不上，照旧用这边的，等下一次再问。

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
        if ask {
            self.ask_bill();
        }
    }

    /// 停放着的会话推来的（`route` 交给停放那一份以前）：是正在看的会话派出去的子代理一轮结束、一轮外来钱的，也问一次。
    pub(super) fn bill_child(&mut self, session: &str, update: &Update) {
        let Update::Push(push) = update else {
            return;
        };
        let ask = match push {
            Push::TurnEnded(_) => true,
            Push::Billed(Some(_)) => self
                .talk
                .parked
                .get(session)
                .is_none_or(|p| p.transcript.running.is_none()),
            _ => false,
        };
        let ours = super::sessions::notes::in_tree(session, &self.talk.board, &self.talk.parked);
        if ask && ours && self.talk.transcript.running.is_none() {
            self.ask_bill();
        }
    }

    /// 问正在看的会话连同它派的子会话一共花了多少，核心换算好的。
    fn ask_bill(&mut self) {
        if let Some(session) = self.talk.transcript.session.clone() {
            self.link.core.send(Command::Ask {
                tag: BILL_TAG,
                method: "usage.query",
                params: json!({ "session": session, "tree": true }),
            });
            self.link.bill_asked = Some(session);
        }
    }

    /// 问的回应归这里交回 `true`：对得上就记下这个会话的那一份（问完又开了一轮的、换了会话的不记）。
    pub(super) fn bill_answer(&mut self, update: &Update) -> bool {
        match update {
            Update::Answer { tag, result } if *tag == BILL_TAG => {
                let asked = self.link.bill_asked.take();
                let same = asked.is_some() && asked == self.talk.transcript.session;
                if let Ok(result) = result
                    && same
                    && self.talk.transcript.running.is_none()
                    && let Some(row) = crate::core::usage_rows(result).into_iter().next()
                    && row.bill.same_amounts(&self.talk.bill_raw())
                    && let Some(session) = asked
                {
                    self.talk.tree_bill = Some((session, row.bill));
                }
                true
            }
            _ => false,
        }
    }
}
