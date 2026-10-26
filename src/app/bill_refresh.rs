//! 底栏、侧边栏的金额换算成一个币种（核心 UC-1，蓝图 `tui.md`「配置与模型」第 5 条）：核心只在订阅回应、`usage.query` 里给
//! 换算好的数，实时推来的 `model.called` 照旧是原币种。实时来的币种对得上的加进总数、对不上的先接在后面（`Converted::put`），
//! 一轮结束问一次这个会话的 `usage.query`，照核心换好的整份换掉，又是一个数。事件触发，不轮询。

use serde_json::json;

use super::App;
use crate::core::{Command, Push, Update};
use crate::oobe::asker::TAG_BASE;

/// 问这个会话合计的那一次。
const BILL_TAG: u64 = TAG_BASE - 21;

impl App {
    /// 一轮结束：问一次。回应归这里交回 `true`：照它换掉这个会话的账（问完又开了一轮的、换了会话的不换）。
    pub(super) fn bill_update(&mut self, update: &Update) -> bool {
        match update {
            Update::Push(Push::TurnEnded(_)) => {
                if let Some(session) = self.talk.transcript.session.clone() {
                    self.link.core.send(Command::Ask {
                        system: false,
                        tag: BILL_TAG,
                        method: "usage.query",
                        params: json!({ "session": session }),
                    });
                    self.link.bill_asked = Some(session);
                }
                false
            }
            Update::Answer { tag, result } if *tag == BILL_TAG => {
                let asked = self.link.bill_asked.take();
                let same = asked.is_some() && asked == self.talk.transcript.session;
                if let Ok(result) = result
                    && same
                    && self.talk.transcript.running.is_none()
                    && let Some(row) = crate::core::usage_rows(result).into_iter().next()
                {
                    self.talk.transcript.bill = row.bill;
                }
                true
            }
            _ => false,
        }
    }
}
