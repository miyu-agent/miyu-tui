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
    /// 这个终端的键（`terminal.rs`）：开的新会话只给这个终端。
    terminal: Option<String>,
}

impl FreshPick {
    pub fn new(words: &Words, terminal: Option<String>) -> Self {
        Self {
            done: None,
            words: words.clone(),
            terminal,
        }
    }
}

impl Picker for FreshPick {
    fn needs_session(&self) -> bool {
        false
    }

    fn start(&mut self, core: &Core) {
        let cwd = std::env::current_dir()
            .ok()
            .map(|d| d.to_string_lossy().into_owned());
        core.send(Command::Ask {
            tag: FRESH_TAG,
            method: "session.terminal",
            params: with_key(
                json!({ "cwd": cwd, "fresh": true }),
                self.terminal.as_deref(),
            ),
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

/// 参数里写上这个终端的键（有的才写：没有的照账号原来那一个指针）。
pub fn with_key(mut params: serde_json::Value, terminal: Option<&str>) -> serde_json::Value {
    if let Some(key) = terminal {
        params["terminal"] = serde_json::Value::from(key);
    }
    params
}
