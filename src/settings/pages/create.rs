//! 新建预设、人格的两格窗（蓝图「配置页」第 36、39 条子规则，2026-11-02 项目主人：「新建预设的时候没有让设置 id……后续 cli
//! 的时候用起来就会很不方便」，选「先名字后编号，编号可空」，又定「编号改名 id，放在名字上面一行」）：上面一格 id、下面一格
//! 名字；id 空着的照旧让核心起（`preset-<n>`、`persona-<n>`），写了的照核心的写法先查一遍、和已有的重了当场说。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde::Deserialize;
use serde_json::json;

use super::persona_edit::change;
use crate::input::Editor;
use crate::pairs;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts};

/// 两格窗的字（`text/zh.json` 的 `settings.more.create`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateTexts {
    /// 窗的标题：新建预设、新建人格。
    pub titles: [String; 2],
    /// 两格前面的字：id、名字。
    pub sides: [String; 2],
    /// id 空着时那一格暗色写的。
    pub id_hint: String,
    /// 名字空着交的。
    pub name_missing: String,
    /// id 不合写法。
    pub id_bad: String,
    /// id 和已有的重了。
    pub id_taken: String,
    /// 写了 id、已有的列表还没读到（查不了重没重）。
    pub id_unchecked: String,
}

/// 建的是哪一种。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// 预设。
    Preset,
    /// 人格。
    Persona,
}

/// 开着的两格窗。
#[derive(Debug)]
pub struct CreateEdit {
    /// 建哪一种。
    pub kind: Kind,
    /// 两格：id、名字。
    pub sides: [Editor; 2],
    /// 光标在哪一格。
    pub focus: usize,
    /// 交不了的原因。
    pub error: Option<String>,
}

/// id 合不合核心的写法（核心 `miyu-store` 的 `layers::valid`）：小写字母开头，小写字母、数字、`-`、`_`，最多 64 个。
pub fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        && id.len() <= 64
}

impl Settings {
    /// 预设页、人格页 `a`：开两格窗，光标在 id 那一格。
    pub(in crate::settings) fn open_create(&mut self, kind: Kind) {
        self.popup = Some(Popup::Create(CreateEdit {
            kind,
            sides: [Editor::default(), Editor::default()],
            focus: 0,
            error: None,
        }));
    }

    /// 两格窗的按键：`↑` `↓`、`Tab` 换格；在 id 那一格 `Enter` 到名字；在名字那一格 `Enter` 交；`Esc` 不建。
    pub(in crate::settings) fn create_key(
        &mut self,
        mut edit: CreateEdit,
        key: KeyEvent,
        texts: &Texts,
    ) {
        match key.code {
            KeyCode::Esc => return,
            KeyCode::Up | KeyCode::Down | KeyCode::Tab | KeyCode::BackTab => {
                edit.focus = 1 - edit.focus
            }
            KeyCode::Enter if edit.focus == 0 => edit.focus = 1,
            KeyCode::Enter => {
                if let Err((why, focus)) = self.create(&edit, texts) {
                    edit.error = Some(why);
                    edit.focus = focus;
                } else {
                    return;
                }
            }
            _ => {
                pairs::type_into(&mut edit.sides[edit.focus], key);
                edit.error = None;
            }
        }
        self.popup = Some(Popup::Create(edit));
    }

    /// 两格窗里粘贴：进光标那一格（换行不要）。
    pub(in crate::settings) fn create_paste(edit: &mut CreateEdit, text: &str) {
        edit.sides[edit.focus].insert(&text.replace(['\r', '\n'], ""));
    }

    /// 交：名字不能空；id 空着的让核心起，写了的先查写法、查重。成了的交给原来新建那一套（建好开它的窗）；不成的交回为什么、
    /// 光标该停哪一格。
    fn create(&mut self, edit: &CreateEdit, texts: &Texts) -> Result<(), (String, usize)> {
        let words = &texts.more.create;
        let [id, name] = edit.sides.each_ref().map(|e| e.text().trim().to_string());
        if !id.is_empty() && !valid_id(&id) {
            return Err((words.id_bad.clone(), 0));
        }
        let have = match edit.kind {
            Kind::Preset => self.more.presets.as_deref(),
            Kind::Persona => self.more.personas.as_deref(),
        };
        // 列表还没读到的不交：查不了重没重，重了的会改掉已有的那一个（从人格框、预设框按 `a` 进来时一开窗就能交）。
        match have {
            None if !id.is_empty() => return Err((words.id_unchecked.clone(), 0)),
            Some(list) if !id.is_empty() && list.iter().any(|p| p.id == id) => {
                return Err((words.id_taken.clone(), 0));
            }
            _ => {}
        }
        if name.is_empty() {
            return Err((words.name_missing.clone(), 1));
        }
        match edit.kind {
            Kind::Preset => self.new_preset_named(&id, &name),
            Kind::Persona => {
                let body = json!({"changes": [change("persona.name", json!(name))]});
                self.set_persona(&id, body, true);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
