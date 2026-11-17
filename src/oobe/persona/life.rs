//! 建人格那一步的「推演人生」开关（蓝图 `tui.md`「推演人生」第 6 条，核心 LF-5、设计 70 第二节第 10 条）：默认关；这一格只写
//! 开、关，光标在上面按 `Enter` 开一个窗（2026-11-17 项目主人本机验收第 32 条：原来像一格带底色的输入框，按一次在下面写影响、
//! 再按一次才开），窗里写三行影响，在窗里选开、关，`Enter` 定、`Esc` 不改。「下一步」存好人格、设成默认以后照开着的
//! `life.start`。已经开着人生会话的人格在这里不关它（关是暂停，归配置页人格的窗）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::json;

use super::super::asker::{Asker, Waiting};
use super::super::nav::{self, Nav};
use super::{PersonaStep, Slot};

/// 开关在哪一步。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LifePick {
    /// 关着。
    #[default]
    Off,
    /// 开着：存好以后 `life.start`。
    On,
    /// 窗开着：选着开还是关（`on`），开窗以前是什么（`was`，`Esc` 回到它）。
    Asking { on: bool, was: bool },
}

impl LifePick {
    /// 这一格写开还是关：窗开着时照开窗以前的。
    pub fn shown_on(self) -> bool {
        matches!(self, LifePick::On | LifePick::Asking { was: true, .. })
    }

    /// 窗开着时选着的是不是「开」；窗没开的是 `None`。
    pub fn asking(self) -> Option<bool> {
        match self {
            LifePick::Asking { on, .. } => Some(on),
            _ => None,
        }
    }
}

impl PersonaStep {
    /// 光标在这一格时的按键，用掉了的交回 `true`：`Enter` 开窗；窗开着时上下换开、关，`Enter` 定，`Esc` 不改，别的键都归窗。
    pub(super) fn life_key(&mut self, key: KeyEvent) -> bool {
        if self.focus != Slot::Life {
            return false;
        }
        let on_off = |on: bool| if on { LifePick::On } else { LifePick::Off };
        match self.life {
            LifePick::Asking { on, was } => match key.code {
                KeyCode::Enter => self.life = on_off(on),
                KeyCode::Esc => self.life = on_off(was),
                _ if matches!(nav::plain(key), Some(Nav::Up | Nav::Down)) => {
                    self.life = LifePick::Asking { on: !on, was };
                }
                _ => {}
            },
            now if key.code == KeyCode::Enter => {
                let was = now == LifePick::On;
                self.life = LifePick::Asking { on: was, was };
            }
            _ => return false,
        }
        true
    }

    /// 人格存好、设成默认了：开着的开它的人生会话（已经有的核心回 `life_exists`，不管）。
    pub(super) fn start_life(&self, asker: &mut Asker, persona: &str) {
        if self.life == LifePick::On && !persona.is_empty() {
            asker.ask(
                "life.start",
                json!({"persona": persona}),
                Waiting::LifeStart,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::LifePick;
    use crate::oobe::persona::{PersonaStep, Slot};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn enter_opens_a_window_where_on_or_off_is_chosen() {
        let mut step = PersonaStep {
            focus: Slot::Life,
            ..PersonaStep::default()
        };
        assert!(step.life_key(key(KeyCode::Enter)));
        assert_eq!(step.life.asking(), Some(false), "开窗，停在现在的「关」上");
        assert!(step.life_key(key(KeyCode::Char('j'))), "窗开着时按键都归窗");
        assert_eq!(step.life.asking(), Some(true), "上下换到「开」");
        assert!(!step.life.shown_on(), "定以前这一格照旧写关");
        assert!(step.life_key(key(KeyCode::Enter)));
        assert_eq!(step.life, LifePick::On, "Enter 定");
        step.life_key(key(KeyCode::Enter));
        step.life_key(key(KeyCode::Down));
        assert!(step.life_key(key(KeyCode::Esc)));
        assert_eq!(step.life, LifePick::On, "Esc 不改");
        step.life_key(key(KeyCode::Enter));
        step.life_key(key(KeyCode::Up));
        step.life_key(key(KeyCode::Enter));
        assert_eq!(step.life, LifePick::Off, "在窗里选关");
        assert!(
            !step.life_key(key(KeyCode::Char('j'))),
            "窗没开时挪开交出去"
        );
    }
}
