//! 一段人生结束了（蓝图 `tui.md`「推演人生」第 9 条；2026-11-17 项目主人定「人生结束怎么告诉主人」，核心 LF-14）：
//!
//! - 推来的 `life.ended` 只有她自己结束的（`by` 是 `self`）弹系统通知「美羽 的人生结束了」，点开切到那个会话；人结束的不弹。
//!   弹不弹照「系统通知」那一套（有人认领了后台通知的，没开着那个会话的由认领的那一头弹，同定时任务跑完了）。不写遗言。
//! - 她自己结束了、还没人看过的（`life.list` 的 `ended_unseen`）在输入框上面那一块最上面写一行「美羽 的人生结束了」，
//!   后面「打开」「收起」两处点得动，一直留着到看过：打开切过去（核心照 `session.viewing` 清掉），收起调 `life.seen`。连上
//!   核心（订 `notices` 的回应）、推来 `life.ended` 时问一次 `life.list`；推来 `life.seen`（别的头看过了）收起。
//! - 第几轮（`rounds`）不显示。

use ratatui::layout::Position;
use serde_json::json;
use unicode_width::UnicodeWidthStr;

use super::App;
use crate::core::{Command, LifeEnded, Update};
use crate::oobe::asker::TAG_BASE;

/// 问 `life.list` 要 `ended_unseen` 的请求编号。
const ENDED_TAG: u64 = TAG_BASE - 52;
/// `life.seen` 的请求编号。
const SEEN_TAG: u64 = TAG_BASE - 53;

/// 提示那一行的几截：「美羽 的人生结束了」，空两格，「打开」，空两格，「收起」；还有别的没看过的接「（还有 N 段）」。
/// 交回前面那句话、「打开」「收起」各从第几列起、占几列（点它们照这个认，`ui/above.rs` 照同样的空法画）。
pub(crate) fn ended_line(
    first: &LifeEnded,
    more: usize,
    texts: &crate::config::LivesTexts,
) -> (String, [(usize, usize); 2]) {
    let name = first.name.as_deref().unwrap_or("Miyu");
    let mut said = texts.ended[0].replace("{name}", name);
    if more > 0 {
        said.push_str(&texts.ended[1].replace("{n}", &more.to_string()));
    }
    let [open, dismiss] = [&texts.ended[2], &texts.ended[3]];
    let open_at = said.width() + 2;
    let dismiss_at = open_at + open.width() + 2;
    let spots = [(open_at, open.width()), (dismiss_at, dismiss.width())];
    (said, spots)
}

impl App {
    /// 问一次 `life.list`，只要 `ended_unseen`。
    pub(super) fn ask_ended(&mut self) {
        self.link.core.send(Command::Ask {
            tag: ENDED_TAG,
            method: "life.list",
            params: json!({}),
        });
    }

    /// 推来一段人生结束了：重读没看过的；她自己结束的弹系统通知。
    pub(super) fn life_ended(&mut self, session: &str, name: Option<&str>, by_self: bool) {
        self.ask_ended();
        let elsewhere = self.talk.transcript.session.as_deref() != Some(session);
        if !by_self || (self.link.notices_claimed && elsewhere) {
            return;
        }
        let texts = &self.config.text.loops.life;
        let body = texts.ended[0].replace("{name}", name.unwrap_or("Miyu"));
        let who = self.who(self.session_persona(session).as_deref());
        self.notifier
            .tell_finished((None, who.as_ref()), &body, session.to_string());
    }

    /// 那一段看过了（推来的 `life.seen`、自己点了打开、收起）：收起那一行。
    pub(super) fn life_seen(&mut self, session: &str) {
        self.link.ended_unseen.retain(|e| e.session != session);
    }

    /// `life.list`、`life.seen` 的回应；归这里交回 `true`。拒了的（旧核心不认 `ended_unseen`）不说，`life.seen` 拒了弹原话。
    pub(super) fn ended_answer(&mut self, update: &Update) -> bool {
        let Update::Answer { tag, result } = update else {
            return false;
        };
        match (*tag, result) {
            (ENDED_TAG, Ok(got)) => {
                self.link.ended_unseen = crate::core::read_ended(&got["ended_unseen"]);
            }
            (SEEN_TAG, Err(refused)) => self.hint(refused.message.clone(), false),
            (ENDED_TAG | SEEN_TAG, _) => {}
            _ => return false,
        }
        true
    }

    /// 点在输入框上面那一块的第一行（有没看过的结束了的人生时那一行是提示）：点「打开」切过去，点「收起」调 `life.seen`。
    /// 交回点没点在这一行上。
    pub(super) fn ended_clicked(&mut self, at: Position) -> bool {
        let area = self.screen.areas.todo;
        let Some(first) = self.link.ended_unseen.first().cloned() else {
            return false;
        };
        if area.height == 0 || at.y != area.y || !area.contains(at) {
            return false;
        }
        let more = self.link.ended_unseen.len() - 1;
        let (_, [open, dismiss]) = ended_line(&first, more, &self.config.text.loops.life);
        let x = usize::from(at.x - area.x);
        let on = |(from, width): (usize, usize)| (from..from + width).contains(&x);
        if on(open) {
            self.life_seen(&first.session);
            self.switch_session(first.session);
        } else if on(dismiss) {
            self.life_seen(&first.session);
            self.link.core.send(Command::Ask {
                tag: SEEN_TAG,
                method: "life.seen",
                params: json!({ "session": first.session }),
            });
        }
        true
    }
}
