//! 人打的 `/dream`（蓝图 `tui.md`「整理记忆」）：交给核心以后订阅这个会话那一间的记忆流，推来的照 [`Dream`] 算、接在正文
//! 末尾画；回执来了吉祥物醒一下，再收成回执那一行、退订。被拒的（没装记忆、正在整理）收掉，照旧弹原话。

use std::time::{Duration, Instant};

use super::App;
use crate::core::Command;
use crate::dream::{Change, Dream};
use crate::transcript::Kind;

impl App {
    /// 打的那一行交给核心以后：是 `/dream`、开着会话的，跟着看整理（第 1 条）。
    pub(super) fn dream_start(&mut self, line: &str) {
        if line.split_whitespace().next() != Some("/dream") {
            return;
        }
        let Some(session) = self.talk.transcript.session.clone() else {
            return;
        };
        self.talk.dream = Some(Dream::new(session.clone(), Instant::now()));
        self.link.core.send(Command::Dream(Some(session)));
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
        self.link.core.send(Command::Dream(None));
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
            self.link.core.send(Command::Dream(None));
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
        self.talk.dream = None;
        let said = said.trim_end().trim_end_matches(['。', '.']).to_string();
        if said.is_empty() {
            return;
        }
        if here {
            self.talk.transcript.note(Kind::Note, said);
        } else {
            self.hint(said, false);
        }
    }

    /// 下一次要醒来：整理着的照转圈的节拍（z 飘、星星闪），醒够了的到点收。
    pub(super) fn dream_deadline(&self) -> Option<Instant> {
        let dream = self.talk.dream.as_ref()?;
        let beat = Instant::now() + Duration::from_millis(self.config.timeline.spinner_ms);
        let wake = Duration::from_millis(self.config.layout.dream.wake_ms);
        let done = dream.woke.as_ref().map(|(at, _)| *at + wake);
        [Some(beat), done, dream.next_float()]
            .into_iter()
            .flatten()
            .min()
    }
}
