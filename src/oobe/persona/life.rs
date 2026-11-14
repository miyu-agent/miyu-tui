//! 建人格那一步的「推演人生」开关（蓝图 `tui.md`「推演人生」第 6 条，核心 LF-5、设计 70 第二节第 10 条）：默认关；切到开先
//! 在这一格下面写出影响、再按一次 `Enter` 才开（二次确认），`Esc`、挪开都算不开；「下一步」存好人格、设成默认以后照开着的
//! `life.start`。已经开着人生会话的人格在这里不关它（关是暂停，归配置页人格的窗）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::json;

use super::super::asker::{Asker, Waiting};
use super::{PersonaStep, Slot};

/// 开关在哪一步。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LifePick {
    /// 关着。
    #[default]
    Off,
    /// 按过一次 `Enter`：写出影响，等再按一次。
    Asking,
    /// 开着：存好以后 `life.start`。
    On,
}

impl PersonaStep {
    /// 光标在这一格时的按键，用掉了的交回 `true`：`Enter` 关→问→开、开→关；问着时 `Esc` 不开，别的键（挪开）也不开、
    /// 照常交出去。
    pub(super) fn life_key(&mut self, key: KeyEvent) -> bool {
        if self.focus != Slot::Life {
            return false;
        }
        match (key.code, self.life) {
            (KeyCode::Enter, LifePick::Off) => self.life = LifePick::Asking,
            (KeyCode::Enter, LifePick::Asking) => self.life = LifePick::On,
            (KeyCode::Enter, LifePick::On) => self.life = LifePick::Off,
            (KeyCode::Esc, LifePick::Asking) => self.life = LifePick::Off,
            (_, LifePick::Asking) => {
                self.life = LifePick::Off;
                return false;
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
    fn turning_life_on_takes_a_second_enter_and_esc_or_moving_away_cancels() {
        let mut step = PersonaStep {
            focus: Slot::Life,
            ..PersonaStep::default()
        };
        assert!(step.life_key(key(KeyCode::Enter)));
        assert_eq!(step.life, LifePick::Asking, "按一次只是问");
        assert!(step.life_key(key(KeyCode::Esc)));
        assert_eq!(step.life, LifePick::Off, "Esc 不开");
        step.life_key(key(KeyCode::Enter));
        assert!(!step.life_key(key(KeyCode::Char('j'))), "挪开交出去");
        assert_eq!(step.life, LifePick::Off, "挪开也不开");
        step.life_key(key(KeyCode::Enter));
        step.life_key(key(KeyCode::Enter));
        assert_eq!(step.life, LifePick::On, "再按一次才开");
        step.life_key(key(KeyCode::Enter));
        assert_eq!(step.life, LifePick::Off, "开着的再按关");
    }
}
