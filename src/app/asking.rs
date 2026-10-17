//! 核心推来的确认、提问接到抽屉上（蓝图「确认和提问的抽屉」开头一段、第 1、6、7、8 条；核心 D-1、D-2）：来了开抽屉、
//! 这次调用了结了收起；推来的回答写进正文；交了被拒的重新打开、弹一句。

use super::App;
use crate::core::{Asking, Push, Update};
use crate::drawer::{Answered, Approval, Asked, Decided, Drawer, Outcome};
use crate::transcript::Transcript;

impl App {
    /// 先看一眼核心的消息：确认、提问归抽屉（交回 `true`，不再往下走）；工具有了结果的收起它的抽屉（照常往下走）。
    pub(super) fn asking_update(&mut self, update: &Update) -> bool {
        match update {
            Update::Push(push) => {
                let owner = self.talk.main_session().unwrap_or_default();
                self.asking_push(&owner, None, push)
            }
            // 推送一律带着会话编号来（`serve.rs`）：主会话的当主会话的，主会话派出去的（一层层往下）是子会话问的（第 8 条），
            // 别的会话（切走了停放着的）问的先搁着、不弹（第 10 条，2026-10-18 项目主人：原来当成子代理问的、弹在看着的
            // 会话里）。切过去还在补发的那个也是主会话的：补完就换上来（review，2026-10-11）。
            Update::Elsewhere { session, update } => match update.as_ref() {
                Update::Push(push) => {
                    let main = self.talk.main_session().as_deref() == Some(session.as_str())
                        || self.talk.opening(session);
                    if main {
                        return self.asking_push(session, None, push);
                    }
                    if self.talk.family().contains(session) {
                        return self.asking_push(session, Some(session), push);
                    }
                    self.asking_elsewhere(session, push)
                }
                _ => false,
            },
            Update::AnswerRefused {
                session,
                call,
                reason,
                message,
            } => {
                // 已经答过、了结了：不说，抽屉照推送收（第 7 条）。别的重新打开、弹核心的原话。
                if reason.as_deref() != Some("not_asking")
                    && let Some(drawer) = self.talk.asks.again(session, call)
                {
                    self.talk.drawers.reopen(drawer);
                    self.hint(message.clone(), false);
                }
                true
            }
            _ => false,
        }
    }

    /// `owner` 是问的会话的整个编号；`session` 只有子会话问的才有（写谁在问、结果写不写进正文）。
    fn asking_push(&mut self, owner: &str, session: Option<&String>, push: &Push) -> bool {
        match push {
            Push::Asking(asking) => {
                self.asking(owner, session, asking);
                true
            }
            Push::ToolResult { call_id, .. } => {
                self.talk.asks.settle(owner, call_id);
                self.close_drawer(owner, call_id);
                false
            }
            _ => false,
        }
    }

    fn asking(&mut self, owner: &str, session: Option<&String>, asking: &Asking) {
        let who = session.map(|s| {
            let name = self.talk.board.agent_title(s).unwrap_or(s);
            self.config.text.drawer.agent.replace("{name}", name)
        });
        let drawer = match asking {
            Asking::Asked(body) => serde_json::from_value::<Asked>(body.clone())
                .ok()
                .map(|a| Drawer::question(who, a)),
            Asking::Approval(body) => serde_json::from_value::<Approval>(body.clone())
                .ok()
                .map(|a| Drawer::approval(who, a)),
            Asking::Answered(body) => {
                if let Ok(a) = serde_json::from_value::<Answered>(body.clone()) {
                    self.answered(owner, session, &a.call_id.clone(), &Outcome::Answered(a));
                }
                return;
            }
            Asking::Decided(body) => {
                if let Ok(d) = serde_json::from_value::<Decided>(body.clone()) {
                    self.answered(owner, session, &d.call_id.clone(), &Outcome::Decided(d));
                }
                return;
            }
        };
        let Some(mut drawer) = drawer else {
            return;
        };
        drawer.session = session.cloned();
        drawer.owner = owner.to_string();
        // 跑命令的确认：核心的 `detail` 没带短标题、命令的（D-4 以前的核心），照时间线里这一步的参数写。
        let args = session
            .map_or_else(|| self.owned(owner), |_| None)
            .and_then(|t| t.tool_args(&drawer.call_id));
        if drawer.command.is_none()
            && let Some(command) = args.and_then(|a| a["command"].as_str())
        {
            let title = args
                .and_then(|a| a["description"].as_str())
                .map(str::to_string);
            drawer.command = Some((title.filter(|t| !t.trim().is_empty()), command.to_string()));
        }
        if !self.talk.asks.asked(&drawer) {
            return;
        }
        let was_open = self.talk.drawers.open();
        // 补发来的是以前问的：抽屉照开（补完还没了结的就是在等），不弹、不报在等（「系统通知」第 1 条）。
        let replayed = self.owned(owner).is_some_and(Transcript::replaying);
        self.talk.drawers.push(drawer);
        // 新开的抽屉：弹「在等你」（「系统通知」第 1 条）；前一个还没了结的排着，轮到它时再弹。
        if !was_open && !replayed {
            self.notify_drawer();
        }
    }

    /// 核心落了盘的回答（哪个头答的都算）：收起抽屉；主会话的在正文末尾写结果（第 6 条）。
    fn answered(
        &mut self,
        owner: &str,
        session: Option<&String>,
        call_id: &str,
        outcome: &Outcome,
    ) {
        let asked = self.talk.asks.settle(owner, call_id);
        self.close_drawer(owner, call_id);
        let Some(drawer) = asked.filter(|_| session.is_none()) else {
            return;
        };
        // 写进问的那个会话的正文：切过去还在补发的写进停放着的那一份，不写进看着的。
        let texts = &self.config.text.drawer;
        let transcript = if self.talk.transcript.session.as_deref() == Some(owner) {
            Some(&mut self.talk.transcript)
        } else {
            self.talk.parked.get_mut(owner).map(|p| &mut p.transcript)
        };
        if let Some(transcript) = transcript {
            super::drawer::report_into(transcript, &drawer, outcome, texts);
        }
    }

    /// 会话 `owner` 的正文：正在看的、停放着的。
    fn owned(&self, owner: &str) -> Option<&Transcript> {
        if self.talk.transcript.session.as_deref() == Some(owner) {
            return Some(&self.talk.transcript);
        }
        self.talk.parked.get(owner).map(|p| &p.transcript)
    }

    /// 这一次调用的抽屉开着的收起、排着的拿掉；都了结了回到在做或空闲（「系统通知」第 1、6 条）。
    fn close_drawer(&mut self, owner: &str, call_id: &str) {
        let was_open = self.talk.drawers.open();
        if self.talk.drawers.settle(owner, call_id) {
            if self.talk.drawers.open() {
                self.notify_drawer();
            } else if was_open {
                self.settle_state();
            }
        }
    }
}
