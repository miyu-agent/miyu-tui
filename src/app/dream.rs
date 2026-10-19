//! 人打的 `/dream`（蓝图 `tui.md`「整理记忆」）：交给核心以后订阅这个会话那一间的记忆流，推来的照 [`Dream`] 算、接在正文
//! 末尾画；回执来了吉祥物醒一下，再收成回执那一行、退订。被拒的（没装记忆、正在整理）收掉，照旧弹原话。

use std::time::{Duration, Instant};

use super::App;
use crate::core::Command;
use crate::dream::{Change, Dream};
use crate::transcript::Kind;

impl App {
    /// 打的那一行交给核心以前：是 `/dream` 的，跟着看整理（第 1 条）。还没开会话的（首页）核心先开会话，会话编号到了再认上
    /// （[`App::dream_ready`]）；连接那头开好会话、交命令以前先订阅，开头那一批不漏。
    pub(super) fn dream_start(&mut self, line: &str) {
        if line.split_whitespace().next() != Some("/dream") {
            return;
        }
        let session = self.talk.transcript.session.clone().unwrap_or_default();
        self.talk.dream = Some(Dream::new(session, Instant::now()));
        self.link.core.send(Command::Dream(true));
    }

    /// 首页打的 `/dream`：核心替它开的会话到了，认上。
    pub(super) fn dream_ready(&mut self, session: &str) {
        if let Some(dream) = self.talk.dream.as_mut().filter(|d| d.session.is_empty()) {
            dream.session = session.to_string();
        }
    }

    /// 记忆流推来一样：醒了以后的不要。
    pub(super) fn dream_change(&mut self, session: &str, change: Change) {
        if let Some(dream) = self
            .talk
            .dream
            .as_mut()
            .filter(|d| d.session == session && d.woke.is_none())
        {
            dream.apply(change, Instant::now());
        }
    }

    /// `/dream` 的回执来了：醒一下再收（第 6 条），退订。没在看整理的交回 `false`，照旧弹提示。
    pub(super) fn dream_woke(&mut self, said: &str) -> bool {
        let Some(dream) = self.talk.dream.as_mut().filter(|d| d.woke.is_none()) else {
            return false;
        };
        dream.woke = Some((Instant::now(), said.to_string()));
        self.link.core.send(Command::Dream(false));
        true
    }

    /// 交给核心的那一行被拒：整理还没开始的收掉、退订（被拒的多半就是 `/dream` 自己）。
    pub(super) fn dream_refused(&mut self) {
        if self
            .talk
            .dream
            .as_ref()
            .is_some_and(|d| d.batch().is_none())
        {
            self.talk.dream = None;
            self.link.core.send(Command::Dream(false));
        }
    }

    /// 每一拍：到点浮下一条；醒够了收成回执那一行（第 2 条：只留回执）。
    pub(super) fn dream_tick(&mut self) {
        let look = &self.config.layout.dream;
        let (every, wake) = (
            Duration::from_millis(look.float_ms),
            Duration::from_millis(look.wake_ms),
        );
        let Some(dream) = &mut self.talk.dream else {
            return;
        };
        let now = Instant::now();
        dream.tick(now, every);
        let Some((_, said)) = dream.woke.clone().filter(|(at, _)| now >= *at + wake) else {
            return;
        };
        let here = self.talk.transcript.session.as_ref() == Some(&dream.session);
        let kept = dream.kept().to_vec();
        self.talk.dream = None;
        let said = said.trim_end().trim_end_matches(['。', '.']).to_string();
        if said.is_empty() {
            return;
        }
        // 正文里留回执那一行，能点开看真正存下的改、删、摘要（第 6 条）；看着别的会话的照旧弹提示。
        if here {
            self.talk.transcript.note(Kind::Dreamt, said);
            if let Some(entry) = self.talk.transcript.entries.last_mut() {
                entry.kept = kept;
            }
        } else {
            self.hint(said, false);
        }
    }

    /// 下一次要醒来：整理着的照转圈的节拍（z 飘、星星闪），醒够了的到点收。
    pub(super) fn dream_deadline(&self) -> Option<Instant> {
        // 还没认上会话的不画，不用醒。
        let dream = self.talk.dream.as_ref().filter(|d| !d.session.is_empty())?;
        let beat = Instant::now() + Duration::from_millis(self.config.timeline.spinner_ms);
        let wake = Duration::from_millis(self.config.layout.dream.wake_ms);
        let done = dream.woke.as_ref().map(|(at, _)| *at + wake);
        [Some(beat), done, dream.next_float()]
            .into_iter()
            .flatten()
            .min()
    }
}
