//! 从几个选项里勾的列表（蓝图「配置页」第 47a 条，2026-10-21 项目主人照推荐定「照预设功能那样打勾」，起因是核心 5-15 下的
//! `sandbox.private.remove`：从这台机器的出厂私密目录里勾去掉哪几个）。`control` 是 `list`、带 `options` 的开这个窗：一个选项
//! 一行，`[*]` 勾着、`[ ]` 没勾；空格勾上、取消，`j` `k` 挪，`Esc` 收起，动过的照选项的先后整份当场存。

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
        let on = options.iter().map(|(v, _)| now.contains(v)).collect();
        Self {
            key: item.key.clone(),
            title: item.name.clone(),
            options,
            on,
            sel: 0,
            changed: false,
        }
    }

    /// 勾着的那几个的值，照选项的先后。
    pub fn value(&self) -> Value {
        let picked = self
            .options
            .iter()
            .zip(&self.on)
            .filter(|(_, on)| **on)
            .map(|((v, _), _)| v.clone());
        Value::Array(picked.collect())
    }

    /// 按了一个键：交回要不要收起。
    fn key(&mut self, key: KeyEvent) -> bool {
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
            KeyCode::Esc | KeyCode::Char('q') => return true,
            _ => {}
        }
        false
    }
}

impl Settings {
    /// 开窗（`item` 现在的值是 `value`）。
    pub(in crate::settings) fn open_ticks(&mut self, item: &Item, value: &Value) {
        self.popup = Some(Popup::Ticks(TickEdit::open(item, value)));
    }

    /// 窗里按了一个键：收起时动过的整份存。
    pub(in crate::settings) fn ticks_key(
        &mut self,
        mut edit: TickEdit,
        key: KeyEvent,
        texts: &Texts,
    ) {
        if !edit.key(key) {
            self.popup = Some(Popup::Ticks(edit));
        } else if edit.changed {
            let value = edit.value();
            self.write_setting(&edit.key, value, texts);
        } else {
            self.popup = None;
        }
    }
}

#[cfg(test)]
mod tests;
