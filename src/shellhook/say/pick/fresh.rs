//! `/new`：请核心照新开终端会话的规矩开一个、指过去（`session.terminal {cwd, fresh: true}`，核心 2bd2fa8a），留一行暗色的
//! 「开了新会话」（同别的系统提示，2026-10-21 项目主人：原来印的是普通的字）。下一句就说进新的。

use ratatui::crossterm::event::KeyEvent;
use ratatui::text::Line;
use serde_json::json;

use super::{Picked, Picker, result};
use crate::config::Config;
use crate::core::{Command, Core, Update};
use crate::shellhook::say::turn::Turn;
use crate::shellhook::words::Words;
use crate::ui::Chrome;

/// 开新会话那一问的编号。
const FRESH_TAG: u64 = 96;

/// `/new`。
pub struct FreshPick {
    /// 核心回了：留哪一行。
    done: Option<Line<'static>>,
    words: Words,
}

impl FreshPick {
    pub fn new(words: &Words) -> Self {
        Self {
            done: None,
            words: words.clone(),
        }
    }
}

impl Picker for FreshPick {
    fn start(&mut self, core: &Core) {
        let cwd = std::env::current_dir()
            .ok()
            .map(|d| d.to_string_lossy().into_owned());
        core.send(Command::Ask {
            tag: FRESH_TAG,
            method: "session.terminal",
            params: json!({ "cwd": cwd, "fresh": true }),
        });
    }

    fn core(&mut self, update: &Update, _turn: &Turn) {
        if let Update::Answer {
            tag,
            result: answer,
        } = update
            && *tag == FRESH_TAG
        {
            self.done = Some(match answer {
                Ok(_) => result(&self.words.slash_new),
                Err(refusal) => result(&refusal.message),
            });
        }
    }

    fn ready(&mut self, _turn: &mut Turn, _core: &Core, _config: &Config) -> Picked {
        match self.done.take() {
            Some(line) => Picked::Done(Some(line)),
            None => Picked::Stay,
        }
    }

    fn key(&mut self, _key: KeyEvent, _turn: &mut Turn, _core: &Core, _config: &Config) -> Picked {
        Picked::Stay
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
