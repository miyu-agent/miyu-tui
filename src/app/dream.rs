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
        // 只认不带字的 `/dream`：`/dream stop` 是叫停（[`App::dream_stop`]），不是又一趟。
        if !crate::dream::asks_to_dream(line) {
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

    /// 整理时人说了话：把她叫醒，那一块收起，整理转到后台跑完（第 7 条，2026-10-20 项目主人：她在做梦还能回话不合设定）。
    pub(super) fn dream_wake_up(&mut self) {
        let here = self.talk.transcript.session.clone();
        let Some(dream) = self
            .talk
            .dream
            .as_mut()
            // 首页打的、会话还没认上的也算（同 [`App::dreaming_here`]）。
            .filter(|d| !d.awake && d.woke.is_none() && d.shown_in(here.as_deref()))
        else {
            return;
        };
        dream.awake = true;
        let note = self.config.text.dream.backgrounded.clone();
        self.hint(note, false);
    }

    /// 前台、后台在整理的那一趟：叫停（第 7 条，核心 R-22）。合完的批留着，正在合的那一批放弃；整理那条 `/dream` 回「整理已中断」，
    /// 照常收成回执。没在整理的交回 `false`。
    pub(super) fn dream_stop(&mut self) -> bool {
        if !self.talk.dream.as_ref().is_some_and(|d| d.woke.is_none()) {
            return false;
        }
        if let Some(dream) = self.talk.dream.as_mut() {
            dream.stopping = true;
        }
        self.link.core.send(Command::DreamStop);
        true
    }

    /// 前台正在画那一块（叫醒了的、整理完了的不算）：`Esc` 照它叫停（第 7 条）。首页打的、核心替它开的会话还没到的也算：
    /// 叫停排在 `/dream` 后面交给连接那一头，到时会话已经开好（2026-10-20 查出：原来要等会话认上，刚打完 `/dream` 就按
    /// `Esc` 的被吞掉，测试时好时坏）。
    pub(super) fn dreaming_here(&self) -> bool {
        let viewing = self.talk.transcript.session.as_deref();
        self.talk
            .dream
            .as_ref()
            .is_some_and(|d| d.woke.is_none() && d.shown_in(viewing))
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
        // 叫醒过的不画，回执来了当场收（第 7 条）。
        let wake = if dream.awake { Duration::ZERO } else { wake };
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
        // 还没认上会话的、叫醒了转到后台的不画，不用醒。
        let dream = self
            .talk
            .dream
            .as_ref()
            .filter(|d| !d.session.is_empty() && !d.awake)?;
        let beat = Instant::now() + Duration::from_millis(self.config.timeline.spinner_ms);
        let wake = Duration::from_millis(self.config.layout.dream.wake_ms);
        let done = dream.woke.as_ref().map(|(at, _)| *at + wake);
        [Some(beat), done, dream.next_float()]
            .into_iter()
            .flatten()
            .min()
    }
}
