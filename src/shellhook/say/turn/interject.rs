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
    /// 插的那几句在正文里是哪几条（条目编号）：最下面「排着」的只画这几条（`typing.rs`）。
    entries: Vec<u64>,
    /// 请求带上了哪一条为止（`model.called` 的 `seen`、出字时的「听到了」）：带上了的核心不再退回，算落着了（2026-10-23
    /// 验收：她刚开始答就插话、马上按 Ctrl+C，那次请求带上了它、没出字就停了，既没「听到了」也没退回，停在「↳」上不收工）。
    heard: u64,
}

impl Interject {
    /// 还有插的话没落着：这一句先不收工。
    pub fn waiting(&self) -> bool {
        self.sending > 0 || !self.queued.is_empty()
    }

    /// 正文第 `id` 条是这个 shell 这一次插的话。读回的那一页里还挂着「排着」的旧话、别的 shell 插的都不是：画出来再藏
    /// 会一闪（2026-10-23 验收 bash：第一次说话时有东西删了一下）。
    pub fn owns(&self, id: u64) -> bool {
        self.entries.contains(&id)
    }
}

impl Turn {
    /// 正文第 `id` 条是这个 shell 这一次答话时插的话。
    pub fn interjected(&self, id: u64) -> bool {
        self.interject.owns(id)
    }

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
        if let Some(entry) = self.transcript.entries.last() {
            self.interject.entries.push(entry.id);
        }
        self.interject.sending += 1;
        if now {
            core.send(Command::Interrupt { send: true });
        }
    }

    /// 正文退回的字（`take_returned`）不要：终端会话几个 shell 共用，别的 shell 插的话被退回来，这里也收得到（2026-10-23
    /// 实测：命令行里多出一句别处插的话）。这个 shell 自己插的照序号在 [`Turn::follow_interject`] 里放回去。
    pub(super) fn take_back(&mut self) {
        drop(self.transcript.take_returned());
    }

    /// 核心推来的：插的话推回了序号、被听到了、另开了一轮（那一轮接着印，同叫醒的）。
    pub(super) fn follow_interject(&mut self, update: &Update) {
        let mine = self.said_seq.is_some() || self.said.is_empty();
        match update {
            Update::Push(Push::UserMessage(seq)) if self.interject.sending > 0 && mine => {
                self.interject.sending -= 1;
                if *seq > self.interject.heard {
                    self.interject.queued.push(*seq);
                }
            }
            Update::Push(Push::Heard(seen) | Push::Sent { seen, .. }) => {
                self.interject.heard = self.interject.heard.max(*seen);
                self.interject.queued.retain(|s| s > seen);
            }
            // Ctrl+C 打断：排着的被退回来（`message.withdrawn`），不用再等它们落着；这个 shell 插的那几句字放回去，接在提前敲
            // 的字前面、一条一行，答完回到命令行（同全屏放回输入框，2026-10-23 验收：原来没接住，要再按一次 Ctrl+C，那句还
            // 留在核心的队里，下一次说话被一起带上）。正文还没退（这里先看），照序号取字。
            Update::Push(Push::Withdrawn(seqs)) => {
                let mine: Vec<u64> = self
                    .interject
                    .queued
                    .iter()
                    .copied()
                    .filter(|s| seqs.contains(s))
                    .collect();
                self.interject.queued.retain(|s| !seqs.contains(s));
                let mut back: Vec<String> = self
                    .transcript
                    .entries
                    .iter()
                    .filter(|e| e.seq.is_some_and(|s| mine.contains(&s)))
                    .map(|e| e.text.clone())
                    .collect();
                if !back.is_empty() {
                    if !self.typed.is_empty() {
                        back.push(std::mem::take(&mut self.typed));
                    }
                    self.typed = back.join("\n");
                }
            }
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

#[cfg(test)]
mod tests {
    use super::super::Turn;
    use crate::core::{Push, Update};
    use crate::shellhook::say::look::Look;

    #[test]
    fn a_word_the_request_took_is_no_longer_waited_for() {
        // 2026-10-23 验收：她刚开始答就插话、马上按 Ctrl+C。那次请求带上了它、没出字就停了：没有「听到了」也不退回。
        let mut turn = Turn::new("数到三十".into(), None, Look::new(None));
        turn.sent = true;
        turn.said_seq = Some(1);
        turn.interject.sending = 1;
        turn.follow_interject(&Update::Push(Push::UserMessage(3)));
        assert!(turn.interject.waiting());
        turn.follow_interject(&Update::Push(Push::Sent {
            seen: 4,
            changed: false,
            summary: false,
        }));
        assert!(!turn.interject.waiting(), "带上了就落着了");
        // 序号比请求晚推回的也一样。
        turn.interject.sending = 1;
        turn.follow_interject(&Update::Push(Push::UserMessage(4)));
        assert!(!turn.interject.waiting());
    }
}
