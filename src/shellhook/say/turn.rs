//! 这一轮的状态：先问终端会话、接上，再说那句话；照推送认出这句话开的那一轮，那一轮结束就收工。

use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use serde_json::json;

use crate::config::Config;
use crate::core::{Command, Core, Push, Update};
use crate::human::Human;
use crate::transcript::{Link, Starting, Transcript};

/// 读配置项、问终端会话的回应认它（`Command::Ask` 的编号）。
const TIMELINE_TAG: u64 = 1;
/// 同上：终端会话。
const TERMINAL_TAG: u64 = 2;

/// 界面语言照 `ui.language`（同全屏 `app/language.rs`）：写了一种语言的换成它，`auto` 照系统的；界面上的字、运行状态行的
/// 词、工具的显示名都换。
fn language(code: &str, config: &mut Config, core: &Core) {
    let next = match config.language_table.find(code) {
        Some(language) if code != "auto" => language,
        _ => {
            let table = &config.language_table;
            table.detect(|name| std::env::var(name).ok())
        }
    };
    if next == config.language {
        return;
    }
    if let Ok(fresh) = Config::load(&next, code == "auto") {
        config.text = fresh.text;
        config.pulse = fresh.pulse;
        config.language = next.clone();
        core.send(Command::FetchHuman(next.code().to_string()));
    }
}

/// 主循环等的东西。
pub enum Incoming {
    Core(Update),
    Key(Event),
}

/// 这一轮的状态。
pub struct Turn {
    pub transcript: Transcript,
    /// 确认和提问的抽屉。
    pub asking: super::asking::Asking,
    pub human: Human,
    /// 还没发出去的那句话：问到终端会话、接上以后发。
    pending: Option<String>,
    /// 这一句在哪个目录说。
    cwd: Option<String>,
    /// 你说的那句：核心落了盘推回来（`Said`）照它认序号。
    said: String,
    /// 你说的那句落盘的序号。
    said_seq: Option<u64>,
    /// 这句话开的那一轮：只印这一轮的（接上的会话先补发最新一页，那些不印）。
    pub ours: Option<u64>,
    /// 闲置久了核心换了新会话：旧会话最后一次动静的时刻，印一行就拿走。
    pub renewed: Option<jiff::Timestamp>,
    /// 这期间提前敲的字：交还给 shell 的命令行。
    pub typed: String,
    /// 按过 Ctrl+C。
    pub interrupted: bool,
    /// 收尾了。
    pub done: bool,
}

impl Turn {
    /// 要说 `text`，在 `cwd` 说。
    pub fn new(text: String, cwd: Option<String>) -> Self {
        let mut transcript = Transcript::default();
        transcript.user(text.clone(), Vec::new());
        transcript.starting = Starting::Waiting;
        Self {
            transcript,
            asking: super::asking::Asking::default(),
            human: Human::default(),
            pending: Some(text.clone()),
            cwd,
            said: text,
            said_seq: None,
            ours: None,
            renewed: None,
            typed: String::new(),
            interrupted: false,
            done: false,
        }
    }

    /// 问核心终端会话是哪一个（`session.terminal`，要新开的开在 shell 所在的目录）。
    pub fn ask_terminal(&self, core: &Core) {
        core.send(Command::Ask {
            tag: TERMINAL_TAG,
            method: "session.terminal",
            params: json!({ "cwd": self.cwd }),
        });
    }

    /// 那句话发出去：接上了终端会话的说进它，没问到的（核心旧）照新会话开。
    fn send(&mut self, core: &Core) {
        if let Some(text) = self.pending.take() {
            core.send(Command::Send {
                text,
                files: Vec::new(),
                cwd: self.cwd.clone(),
            });
        }
    }

    /// 核心那边来的：终端会话、头的配置、给人看的字归这里，别的交给正文。
    pub fn core(&mut self, update: Update, config: &mut Config, core: &Core) {
        // 终端会话的推送包着会话编号来的照常办（同全屏 `app/sessions.rs` 的 `route`），别的会话的不管。
        let update = match update {
            Update::Elsewhere { session, update }
                if self.transcript.session.as_deref() == Some(session.as_str()) =>
            {
                *update
            }
            Update::Elsewhere { .. } => return,
            update => update,
        };
        self.watch(&update);
        if let Update::Push(push) = &update {
            let owner = self.transcript.session.clone().unwrap_or_default();
            if self.asking.push(&owner, push, &mut self.transcript, config) {
                return;
            }
        }
        match update {
            Update::Answer { tag, result } if tag == TERMINAL_TAG => {
                if let Some(found) = result.ok().as_ref().and_then(super::terminal::read) {
                    self.renewed = found.renewed;
                    self.transcript.session = Some(found.session.clone());
                    self.transcript.link = Link::Ready;
                    core.send(Command::Open {
                        session: found.session,
                        keep: false,
                    });
                }
                self.send(core);
            }
            Update::HeadConfig(head) => {
                language(&head.language, config, core);
                core.send(Command::Ask {
                    tag: TIMELINE_TAG,
                    method: "config.get",
                    params: json!({ "keys": crate::config::Timeline::KEYS }),
                });
            }
            Update::Answer { tag, result } if tag == TIMELINE_TAG => {
                if let Ok(got) = result {
                    config.timeline.apply(&got);
                }
            }
            Update::Human(human) => self.human = human,
            update @ (Update::NoCoreBin
            | Update::Failed(_)
            | Update::Missing(_)
            | Update::SessionGone(_)) => {
                self.transcript.update(update, &config.text);
                self.done = true;
            }
            update @ (Update::Refused { .. } | Update::Unsent { .. }) => {
                self.transcript.update(update, &config.text);
                if self.ours.is_none() {
                    self.done = true;
                }
            }
            update => self.transcript.update(update, &config.text),
        }
        // 这句话开的那一轮结束了：收工。
        if self.ours.is_some() && self.transcript.running.is_none() {
            self.done = true;
        }
    }

    /// 认出你说的那句落盘的序号、它开的那一轮。
    fn watch(&mut self, update: &Update) {
        match update {
            Update::Push(Push::Said { seq, text, .. }) if *text == self.said => {
                self.said_seq = Some(*seq);
            }
            Update::Push(Push::TurnStarted(turn, Some(trigger)))
                if Some(*trigger) == self.said_seq =>
            {
                self.ours = Some(*turn);
            }
            _ => {}
        }
    }

    /// 按键：Ctrl+C 打断这一轮（还没开轮的、再按一下的不等了），提前敲的字记下交还给 shell。交回要不要马上收工。
    pub fn key(&mut self, event: &Event, core: &Core, config: &Config) -> bool {
        match event {
            Event::Key(key)
                if key.kind != KeyEventKind::Release
                    && self.asking.key(*key, core, &mut self.transcript, config) => {}
            Event::Paste(text) if self.asking.open() => {
                self.asking.paste(text);
            }
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                let alt = key.modifiers.contains(KeyModifiers::ALT);
                match key.code {
                    KeyCode::Char('c') if ctrl => {
                        let give_up = self.interrupted || self.ours.is_none();
                        self.interrupted = true;
                        if give_up {
                            return true;
                        }
                        core.send(Command::Interrupt { send: false });
                    }
                    KeyCode::Char(c) if !ctrl && !alt => self.typed.push(c),
                    KeyCode::Backspace => {
                        self.typed.pop();
                    }
                    _ => {}
                }
            }
            Event::Paste(text) => self.typed.push_str(text),
            _ => {}
        }
        false
    }
}

#[cfg(test)]
mod tests;
