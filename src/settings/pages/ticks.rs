//! 从几个选项里勾的列表（蓝图「配置页」第 47a 条，2026-10-21 项目主人照推荐定「照预设功能那样打勾」，起因是核心 5-15 下的
//! `sandbox.private.remove`，同一天改名「默认私密目录」）。`control` 是 `list`、带 `options` 的开这个窗：一个选项一行，`[*]`
//! 勾着、`[ ]` 没勾；空格勾上、取消，`j` `k` 挪，`Enter` 存了关窗、`Esc` 不存（同一天项目主人：「空格取消勾选，回车保存」），
//! 动过的照选项的先后整份存。核心标了 `"ticks": "unlisted"` 的反着勾：勾着的是不在值里的（默认私密目录：勾着的还是私密的）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde::Deserialize;
use serde_json::Value;

use super::schema::Item;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts};

/// 勾选窗的字（`text/zh.json` 的 `settings.more.ticks`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TicksTexts {
    /// 一个选项都没有。
    pub empty: String,
    /// 下面的按键提示。
    pub hints: Vec<[String; 2]>,
}

/// 开着的勾选窗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickEdit {
    /// 改的是哪一项。
    pub key: String,
    /// 标题（这一项的名字）。
    pub title: String,
    /// 选项：值、给人看的名字，照核心给的先后。
    pub options: Vec<(Value, String)>,
    /// 每个选项勾没勾。
    pub on: Vec<bool>,
    /// 选中第几行。
    pub sel: usize,
    /// 动过。
    pub changed: bool,
    /// 反着勾：勾着的是不在值里的（`"ticks": "unlisted"`）。
    pub unlisted: bool,
}

/// 按了一个键以后窗怎么办。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Close {
    /// 接着开着。
    Stay,
    /// 收起，动过的存。
    Save,
    /// 收起，不存。
    Drop,
}

impl TickEdit {
    /// 开窗：现在的值（`value`）里有的勾上；不在选项里的不认（核心只收选项里的，写了回 `not_an_option`）。
    pub fn open(item: &Item, value: &Value) -> Self {
        let now = value.as_array().cloned().unwrap_or_default();
        let options: Vec<(Value, String)> = item
            .options
            .iter()
            .map(|o| (o.value.clone(), o.name.clone()))
            .collect();
        let on = options
            .iter()
            .map(|(v, _)| now.contains(v) != item.unlisted)
            .collect();
        Self {
            key: item.key.clone(),
            title: item.name.clone(),
            options,
            on,
            sel: 0,
            changed: false,
            unlisted: item.unlisted,
        }
    }

    /// 存进去的值，照选项的先后：勾着的那几个；反着勾的是没勾的那几个。
    pub fn value(&self) -> Value {
        let picked = self
            .options
            .iter()
            .zip(&self.on)
            .filter(|(_, on)| **on != self.unlisted)
            .map(|((v, _), _)| v.clone());
        Value::Array(picked.collect())
    }

    /// 按了一个键：交回收不收起、收起时存不存。
    fn key(&mut self, key: KeyEvent) -> Close {
        let last = self.options.len().saturating_sub(1);
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.sel = (self.sel + 1).min(last),
            KeyCode::Char('k') | KeyCode::Up => self.sel = self.sel.saturating_sub(1),
            KeyCode::Char(' ') => {
                if let Some(on) = self.on.get_mut(self.sel) {
                    *on = !*on;
                    self.changed = true;
                }
            }
            KeyCode::Enter => return Close::Save,
            KeyCode::Esc | KeyCode::Char('q') => return Close::Drop,
            _ => {}
        }
        Close::Stay
    }
}

impl Settings {
    /// 开窗（`item` 现在的值是 `value`）。
    pub(in crate::settings) fn open_ticks(&mut self, item: &Item, value: &Value) {
        self.popup = Some(Popup::Ticks(TickEdit::open(item, value)));
    }

    /// 窗里按了一个键：`Enter` 收起，动过的整份存；`Esc` 收起不存。
    pub(in crate::settings) fn ticks_key(
        &mut self,
        mut edit: TickEdit,
        key: KeyEvent,
        texts: &Texts,
    ) {
        match edit.key(key) {
            Close::Stay => self.popup = Some(Popup::Ticks(edit)),
            Close::Save if edit.changed => {
                let value = edit.value();
                self.write_setting(&edit.key, value, texts);
            }
            Close::Save | Close::Drop => self.popup = None,
        }
    }
}

#[cfg(test)]
mod tests;
