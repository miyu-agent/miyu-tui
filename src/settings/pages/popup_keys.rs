//! 新几页的悬浮窗的按键和写回（蓝图「配置页」第 33 到 41、45 到 47 条）：选择窗、编辑窗、几行字的窗、列表窗、勾选窗、人格和
//! 预设的窗；选好了照 [`Target`](super::target::Target) 写回去（配置项、预设、人格、列表的一条……），配置项写系统配置。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::choose::Pressed;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts, Waiting};

impl Settings {
    /// 选择窗、编辑窗、详情窗的按键。
    pub(in crate::settings) fn more_popup_key(
        &mut self,
        popup: Popup,
        key: KeyEvent,
        texts: &Texts,
    ) {
        match popup {
            Popup::Choose(mut choose) => match choose.press(key) {
                Pressed::Picked(value) => self.write_to(choose.target, &choose.key, value, texts),
                Pressed::Cancelled => self.edit_cancel(choose.target),
                Pressed::Moved | Pressed::Ignored => self.popup = Some(Popup::Choose(choose)),
            },
            Popup::Line(mut edit) => {
                match key.code {
                    KeyCode::Esc => return self.edit_cancel(edit.target),
                    KeyCode::Enter => {
                        // 列表的一条（第 47 条）：清空了是去掉这一条，整数的写了别的照整数说。
                        let item = matches!(edit.target, super::target::Target::ListItem(..));
                        let blank = edit.editor.text().trim().is_empty();
                        match edit.value() {
                            _ if item && blank => {
                                let value = Value::String(String::new());
                                return self.write_to(edit.target, &edit.key, value, texts);
                            }
                            Some(value) => {
                                return self.write_to(edit.target, &edit.key, value, texts);
                            }
                            None if item => edit.error = Some(texts.more.list.whole.clone()),
                            None => edit.error = Some(texts.more.bad_number.clone()),
                        }
                    }
                    _ => {}
                }
                let editor = &mut edit.editor;
                match key.code {
                    KeyCode::Backspace => editor.backspace(),
                    KeyCode::Delete => editor.delete(),
                    KeyCode::Left => editor.left(false),
                    KeyCode::Right => editor.right(false),
                    KeyCode::Home => editor.move_to(0, false),
                    KeyCode::End => editor.move_to(editor.text().len(), false),
                    KeyCode::Char(c) => editor.insert(&c.to_string()),
                    _ => {}
                }
                self.popup = Some(Popup::Line(edit));
            }
            Popup::Sheet(edit) => self.sheet_key(edit, key, texts),
            Popup::List(edit) => self.list_key(edit, key, texts),
            // 记录列表的列表窗、表单窗（第 47b 条）。
            Popup::Records(edit) => self.records_key(edit, key, texts),
            Popup::Record(form) => self.record_key(form, key, texts),
            Popup::Ticks(edit) => self.ticks_key(edit, key, texts),
            // 人格的窗（第 36 条）、示范对话列表和两格窗（第 41 条）。
            Popup::Persona(view) => self.persona_key(view, key, texts),
            Popup::Dialogs(view) => self.dialogs_key(view, key),
            Popup::Pair(edit) => self.pair_key(edit, key, texts),
            Popup::Create(edit) => self.create_key(edit, key, texts),
            // 能改的预设详情窗（第 39 条）。
            Popup::Preset(view) => self.preset_key(view, key, texts),
            other => self.popup = Some(other),
        }
    }

    /// 选择窗、编辑窗选好了：改预设的照它办（`preset_edit.rs`），别的写系统配置。
    pub(in crate::settings) fn write_to(
        &mut self,
        target: super::target::Target,
        key: &str,
        value: Value,
        texts: &Texts,
    ) {
        // 跟随头像的主色写在人格上（`persona.set` 的 `seed`）。
        if let super::target::Target::PersonaSeed(persona) = &target {
            let params = json!({"persona": persona, "seed": value});
            self.ask("persona.set", params, Waiting::PersonaSeed);
            return;
        }
        // 记录列表一条里的字段（第 47b 条）：写回表单窗。
        if self.record_write(&target, value.clone()) {
            return;
        }
        if !self.model_or_write(&target, key, &value)
            && !self.list_write(&target, &value, texts)
            && !self.secret_write(&target, key, &value, texts)
            && !self.combo_write(&target, key, &value)
            && !self.preset_write(target.clone(), key, value.clone())
            && !self.avatar_write(target.clone(), &value)
            && !self.persona_write(target, key, value.clone())
        {
            self.write_setting(key, value, texts);
        }
    }

    /// 写一项（不写层，核心写系统配置，「设置只剩一份」第 1 条），当场存（关窗；没存成的原因写在状态行）。
    pub(in crate::settings) fn write_setting(&mut self, key: &str, value: Value, texts: &Texts) {
        let Some(data) = self.data.as_ref() else {
            return;
        };
        // 选了「无人格」这类空的、填的字清空了的：去掉这一项（2026-10-26 项目主人报：「用户名称」删空了存不进，核心不收空字）。
        if value.is_null() || value.as_str().is_some_and(|t| t.trim().is_empty()) {
            self.draft.unset(key, data);
        } else {
            self.draft.set(key, value, data);
        }
        self.refresh();
        self.save(false, texts);
    }
}
