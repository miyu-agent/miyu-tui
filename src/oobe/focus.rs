//! 吉祥物照什么反应（蓝图 `tui.md`「第一次打开的引导」第 10 条）：光标在不在一个正在编辑的格子里，打字时歪着头看
//! （`motion/react.rs`）。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{Oobe, Step, model};

impl Oobe {
    /// 在打字（输入法照它切，「配置页」第 30 条）：同 [`Oobe::typing`]。
    pub fn editing(&self) -> bool {
        self.typing()
    }

    /// 在一个格子里编辑着（按 `Enter` 进去的格子、搜着的、大编辑浮窗、两格窗）。
    pub(super) fn typing(&self) -> bool {
        use model::Phase;
        match self.step {
            Step::Model if self.model.more.is_some() => true,
            Step::Model => match &self.model.phase {
                Phase::Form(form) => form.editing(),
                Phase::Models(models) => models.filter.editing(),
                _ => false,
            },
            // 每一格回车进去写字都算（2026-10-26 项目主人报：名字算了、简单描述没算，进第二格输入法停在英文）。
            Step::Persona => {
                let p = &self.persona;
                p.writing.is_some()
                    || p.editing.is_some()
                    || [&p.name, &p.summary, &p.avatar].iter().any(|f| f.editing())
            }
            Step::Profile => {
                let p = &self.profile;
                p.writing.is_some() || p.name.editing() || p.language.editing()
            }
            Step::Register => self.register.typing(),
            Step::Preset => self.preset.custom_open && self.preset.name.editing(),
            _ => false,
        }
    }
}

/// 这个键是打了字、删了字。
pub(super) fn is_typed(key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char(_) => !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT),
        KeyCode::Backspace | KeyCode::Delete => true,
        _ => false,
    }
}
