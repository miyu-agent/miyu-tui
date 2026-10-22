//! 这一轮里的按键：Ctrl+C 打断（在整理的先叫停），提前敲的字记下交还给 shell，抽屉开着的归抽屉。

use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use super::Turn;
use crate::config::Config;
use crate::core::{Command, Core};

impl Turn {
    /// 收工以后才处理到的按键：只记字，不再打断、不再答抽屉。
    pub fn keep_typed(&mut self, event: &Event) {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                let plain = !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
                match key.code {
                    KeyCode::Char(c) if plain => self.typed.push(c),
                    KeyCode::Backspace => {
                        self.typed.pop();
                    }
                    _ => {}
                }
            }
            Event::Paste(text) => self.typed.push_str(text),
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
                    // 在整理的头一下叫停（`dreaming.rs`）。
                    KeyCode::Char('c') if ctrl && super::super::dreaming::stop(self, core) => {}
                    KeyCode::Char('c') if ctrl => {
                        let give_up = self.interrupted || self.ours.is_none();
                        self.interrupted = true;
                        if give_up {
                            return true;
                        }
                        core.send(Command::Interrupt { send: false });
                    }
                    // 回车插话，Ctrl+回车当场打断、插的马上发（`interject.rs`）。
                    KeyCode::Enter => self.interject(core, ctrl),
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
