//! `/undo`、`/restore`、`/recap`（同终端界面，核心 `session.revert`、`session.unrevert`、`session.recap`）：接上终端会话、
//! 订阅上以后交给核心，核心推回来的照说话时的样子印（撤销那一行、回顾）；恢复的那几轮在终端会话里显示回来了，shell 里
//! 留一句暗色的「恢复了」。核心拒了（没有能撤的、在回答）照它的原话说一句。

use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::text::Line;

use super::{Picked, Picker, result};
use crate::config::Config;
use crate::core::{Command, Core, Push, Update};
use crate::shellhook::say::turn::Turn;
use crate::shellhook::words::Words;
use crate::ui::Chrome;

/// 最新一页读回来以后，订阅的回应最多再等多久（同 `/status`）。
const SUBSCRIBED_WAIT: Duration = Duration::from_millis(500);

/// 办哪一件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// `/undo`。
    Revert,
    /// `/restore`。
    Unrevert,
    /// `/recap`。
    Recap,
}

/// 交给核心、等它回。
pub struct RunPick {
    act: Act,
    paged: Option<Instant>,
    subscribed: bool,
    sent: bool,
    end: Option<Picked>,
    words: Words,
}

impl RunPick {
    pub fn new(act: Act, words: &Words) -> Self {
        Self {
            act,
            paged: None,
            subscribed: false,
            sent: false,
            end: None,
            words: words.clone(),
        }
    }
}

impl Picker for RunPick {
    fn start(&mut self, _core: &Core) {}

    fn core(&mut self, update: &Update, _turn: &Turn) {
        match update {
            Update::Paged(_) | Update::Older { .. } => {
                self.paged.get_or_insert_with(Instant::now);
            }
            Update::Snapshot(_) | Update::Limits(_) => self.subscribed = true,
            Update::Undone { restore: false, .. } if self.sent => self.end = Some(Picked::Print),
            Update::Undone { restore: true, .. } if self.sent => {
                self.end = Some(Picked::Done(Some(result(&self.words.slash_restored))));
            }
            Update::Recap(_) | Update::Push(Push::Recapped(_)) if self.sent => {
                self.end = Some(Picked::Print);
            }
            Update::Refused { message, .. } if self.sent => {
                self.end = Some(Picked::Done(Some(result(message))));
            }
            _ => {}
        }
    }

    fn ready(&mut self, _turn: &mut Turn, core: &Core, _config: &Config) -> Picked {
        if let Some(end) = self.end.take() {
            return end;
        }
        let Some(paged) = self.paged else {
            return Picked::Stay;
        };
        if !self.sent && (self.subscribed || paged.elapsed() >= SUBSCRIBED_WAIT) {
            self.sent = true;
            core.send(match self.act {
                Act::Revert => Command::Revert,
                Act::Unrevert => Command::Unrevert,
                Act::Recap => Command::Recap,
            });
        }
        Picked::Stay
    }

    fn key(&mut self, key: KeyEvent, _turn: &mut Turn, _core: &Core, _config: &Config) -> Picked {
        match key.code {
            KeyCode::Esc => Picked::Done(None),
            _ => Picked::Stay,
        }
    }

    fn view(
        &self,
        _turn: &Turn,
        _config: &Config,
        _width: u16,
        _max: usize,
    ) -> (Chrome, Vec<Line<'static>>) {
        (Chrome::new("", Vec::new()), Vec::new())
    }
}
