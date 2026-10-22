//! 她答话时插话（2026-10-22 项目主人：「回车当插话吧」「ctrl+回车打断当前直接发送」；同终端界面答话时发的话排进队、
//! `Ctrl+Enter` 当场打断，`tui.md`「按键」）：提前敲的字回车交给核心排进队，她下一次请求时听到；这一轮先结束了的，核心
//! 拿它另开一轮，接着印。Ctrl+回车排进队以后当场打断，排着的马上发。插的话没落着（没被听到、没开轮）以前这一句不收工。

use super::Turn;
use crate::core::{Command, Core, Push, Update};

/// 插的话落到哪了。
#[derive(Debug, Default)]
pub struct Interject {
    /// 发出去了、序号还没推回来的几句。
    sending: usize,
    /// 推回了序号、还没被听到、也没开轮的。
    queued: Vec<u64>,
}

impl Interject {
    /// 还有插的话没落着：这一句先不收工。
    pub fn waiting(&self) -> bool {
        self.sending > 0 || !self.queued.is_empty()
    }
}

impl Turn {
    /// 回车：提前敲的字插进这一轮；`now`（Ctrl+回车）当场打断，排着的马上发。她没在答的不插，字留着交还给 shell。
    pub(super) fn interject(&mut self, core: &Core, now: bool) {
        let text = self.typed.trim().to_string();
        if text.is_empty() || self.transcript.running.is_none() || !self.live() {
            return;
        }
        self.typed.clear();
        core.send(Command::Send {
            text: text.clone(),
            files: Vec::new(),
            cwd: self.cwd.clone(),
        });
        // 她在答：正文里记成排着的（`queued`），听到了再挪进这一轮（同全屏）。
        self.transcript.user(text, Vec::new());
        self.interject.sending += 1;
        if now {
            core.send(Command::Interrupt { send: true });
        }
    }

    /// 核心推来的：插的话推回了序号、被听到了、另开了一轮（那一轮接着印，同叫醒的）。
    pub(super) fn follow_interject(&mut self, update: &Update) {
        let mine = self.said_seq.is_some() || self.said.is_empty();
        match update {
            Update::Push(Push::UserMessage(seq)) if self.interject.sending > 0 && mine => {
                self.interject.sending -= 1;
                self.interject.queued.push(*seq);
            }
            Update::Push(Push::Heard(seen)) => self.interject.queued.retain(|s| s > seen),
            Update::Push(Push::TurnStarted(turn, Some(trigger)))
                if self.interject.queued.contains(trigger) =>
            {
                // 开轮时排着的一起带上。
                self.interject.queued.clear();
                self.woken.push(*turn);
            }
            Update::Refused { .. } | Update::Unsent { .. } if self.interject.sending > 0 => {
                self.interject.sending -= 1;
            }
            _ => {}
        }
    }
}
