//! 几行字的框（蓝图「配置页」第 45 条，`control` 是 `textarea`，核心 P-9 三补：「用户」页的「更多描述」）：`Enter` 开多行
//! 编辑窗，样子和按键照引导的大编辑浮窗（`oobe/sheet.rs`，2026-10-16 项目主人照推荐定）：`Enter` 换行，`Esc` 收起、改过的
//! 当场存，`Ctrl+G` 交给外部编辑器、回来接着在窗里。

use ratatui::crossterm::event::KeyEvent;
use serde_json::Value;

use super::schema::Item;
use crate::oobe::sheet::{Sheet, SheetKey};
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts};

/// 开着的多行编辑窗。
#[derive(Debug)]
pub struct SheetEdit {
    /// 改的是哪一项。
    pub key: String,
    /// 标题（这一项的名字）。
    pub title: String,
    /// 字、光标、折行。
    pub sheet: Sheet,
}

impl Settings {
    /// 开窗（`item` 现在的值是 `value`）。
    pub(in crate::settings) fn open_textarea(&mut self, item: &Item, value: &Value) {
        let text = value.as_str().unwrap_or_default();
        self.popup = Some(Popup::Sheet(SheetEdit {
            key: item.key.clone(),
            title: item.name.clone(),
            sheet: Sheet::open(text),
        }));
    }

    /// 窗里按了一个键：`Esc` 收起（改过的存，清空了的去掉），`Ctrl+G` 交给外部编辑器，别的照常打字。
    pub(in crate::settings) fn sheet_key(
        &mut self,
        mut edit: SheetEdit,
        key: KeyEvent,
        texts: &Texts,
    ) {
        match edit.sheet.key(key) {
            SheetKey::Stay => self.popup = Some(Popup::Sheet(edit)),
            SheetKey::External => self.hand_out(edit),
            SheetKey::Close => {
                // 末尾的空白、换行不要（编辑器常补一个）；和原来一样的配置页那一层不存（`draft`）。
                let text = edit.sheet.text().trim_end().to_string();
                let value = if text.trim().is_empty() {
                    Value::Null
                } else {
                    Value::String(text)
                };
                self.write_setting(&edit.key, value, texts);
            }
        }
    }

    /// 窗里粘贴：留着换行。
    pub(in crate::settings) fn sheet_paste(edit: &mut SheetEdit, text: &str) {
        edit.sheet.paste(text);
    }

    /// 编辑器写回来了：换上写的字，回到窗里接着改（没读回来的照原样）。
    pub(in crate::settings) fn sheet_back(&mut self, text: Option<String>) {
        let Some(mut edit) = self.sheet_out.take() else {
            return;
        };
        if let Some(text) = text {
            edit.sheet.replace(text.trim_end());
        }
        self.popup = Some(Popup::Sheet(edit));
    }

    /// 交给外部编辑器：记下这个窗，`App` 照 [`Settings::take_prompt`] 拿字。
    fn hand_out(&mut self, edit: SheetEdit) {
        self.sheet_out = Some(edit);
        self.editing = true;
    }

    /// 交出去的那一份字（`take_prompt` 用）。
    pub(in crate::settings) fn sheet_text(&self) -> Option<String> {
        self.sheet_out.as_ref().map(|e| e.sheet.text().to_string())
    }
}

#[cfg(test)]
mod tests;
