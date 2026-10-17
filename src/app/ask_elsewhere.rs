//! 别的会话问的（蓝图「确认和提问的抽屉」第 10 条，2026-10-18 项目主人）：不是正在看的这棵会话树（主会话和它派出去的
//! 子代理）问的确认、提问不弹抽屉，搁着；提示一句「「标题」在问你 · /resume 切过去」、照常弹系统通知，会话列表里那一项标
//! 「在问」；切过去时放回来弹（`switch.rs`）。原来一律当成子代理问的，弹在看着的会话里。

use super::App;
use crate::core::{Asking, Push};
use crate::drawer::{Answered, Approval, Asked, Decided, Drawer};
use crate::notify::Event;

impl App {
    /// 别的会话 `owner` 推来的：问的搁着，答了、有了结果的拿掉。交回 `true` 的不再往下走。
    pub(super) fn asking_elsewhere(&mut self, owner: &str, push: &Push) -> bool {
        let call = match push {
            Push::Asking(Asking::Answered(body)) => {
                serde_json::from_value::<Answered>(body.clone())
                    .ok()
                    .map(|a| a.call_id)
            }
            Push::Asking(Asking::Decided(body)) => serde_json::from_value::<Decided>(body.clone())
                .ok()
                .map(|d| d.call_id),
            Push::ToolResult { call_id, .. } => Some(call_id.clone()),
            Push::Asking(asking) => {
                self.hold_elsewhere(owner, asking);
                return true;
            }
            _ => return false,
        };
        if let Some(call) = call {
            self.talk.asks.settle(owner, &call);
            self.talk.drawers.settle(owner, &call);
        }
        matches!(push, Push::Asking(_))
    }

    fn hold_elsewhere(&mut self, owner: &str, asking: &Asking) {
        let drawer = match asking {
            Asking::Asked(body) => serde_json::from_value::<Asked>(body.clone())
                .ok()
                .map(|a| Drawer::question(None, a)),
            Asking::Approval(body) => serde_json::from_value::<Approval>(body.clone())
                .ok()
                .map(|a| Drawer::approval(None, a)),
            Asking::Answered(_) | Asking::Decided(_) => None,
        };
        let Some(mut drawer) = drawer else {
            return;
        };
        drawer.owner = owner.to_string();
        if !self.talk.asks.asked(&drawer) {
            return;
        }
        let question = drawer.is_question();
        self.talk.drawers.hold(drawer);
        // 补发来的是以前问的：不提示、不弹通知（同抽屉）。
        let replayed = self
            .talk
            .parked
            .get(owner)
            .is_some_and(|p| p.transcript.replaying());
        if replayed {
            return;
        }
        // 没起名的写短编号（会话列表里没见过的顺手要一次，`sessions.rs`）。
        let title = self
            .session_title(owner)
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| crate::session_list::short(owner));
        let note = self.config.text.drawer.elsewhere.replace("{title}", &title);
        self.hint(note, false);
        // 通知的标题写问的那个会话的（「系统通知」第 3 条）；下一次通知照正在看的重新写。
        self.notifier.about(Some(title), None);
        self.notifier.tell(if question {
            Event::Asking
        } else {
            Event::Approving
        });
    }
}
