//! 列表（蓝图「配置页」第 47 条，`control` 是 `list`；2026-10-17 项目主人照推荐定「弹窗逐条改」，起因是接入通讯平台 的群白名单、
//! 群里的触发词）：`Enter` 开窗，一条一行；`a` 加、`Enter` 改（一行字的编辑窗）、`d` 删、`J` `K` 挪，`Esc` 收起，动过的整份
//! 当场存。按键和示范对话列表共用 [`crate::pairs::list_key`]。终端不认识哪个包，照 `config.schema` 的类型画。

use ratatui::crossterm::event::KeyEvent;
use serde::Deserialize;
use serde_json::Value;

use super::choose::LineEdit;
use super::schema::Item;
use super::target::Target;
use crate::pairs::{self, ListKey};
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts, Tone};

/// 配置页上只写个数、不把每一条铺出来的列表（2026-10-22 项目主人：私密目录一长串路径不该铺出来，写个数；别的列表照旧列）。
const COUNTED: [&str; 2] = ["sandbox.private.add", "sandbox.private.remove"];

/// 这一项在配置页上只写个数的话写几个：反着勾的（默认私密目录，`"ticks": "unlisted"`）数勾着的，就是选项里不在值里的。
pub fn counted(item: &Item, value: &Value) -> Option<usize> {
    if !COUNTED.contains(&item.key.as_str()) {
        return None;
    }
    let list = value.as_array().map(Vec::as_slice).unwrap_or_default();
    Some(if item.unlisted {
        item.options
            .iter()
            .filter(|o| !list.contains(&o.value))
            .count()
    } else {
        list.len()
    })
}

/// 列表窗的字（`text/zh.json` 的 `settings.more.list`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListTexts {
    /// 一条都没有。
    pub empty: String,
    /// 元素是整数的，写了别的。
    pub whole: String,
    /// 下面的按键提示。
    pub hints: Vec<[String; 2]>,
}

/// 开着的列表窗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListEdit {
    /// 改的是哪一项。
    pub key: String,
    /// 标题（这一项的名字）。
    pub title: String,
    /// 元素是整数。
    pub whole: bool,
    /// 一条条，照先后。
    pub items: Vec<Value>,
    /// 选中第几条。
    pub sel: usize,
    /// 动过（加、改、删、挪）。
    pub changed: bool,
}

impl ListEdit {
    /// 一条写成的字：字照原样，数照十进制。
    pub fn label(value: &Value) -> String {
        value
            .as_str()
            .map_or_else(|| value.to_string(), str::to_string)
    }
}

/// 这种元素的列表开得了窗：一条写一行字的那几种（字、整数、名字、网址）；开关、选项、密钥的列表不开（第 47 条）。
pub fn editable(item: &Item) -> bool {
    matches!(
        item.element.as_str(),
        "text" | "int" | "name" | "url" | "english"
    )
}

impl Settings {
    /// 开窗（`item` 现在的值是 `value`）。
    pub(in crate::settings) fn open_list(&mut self, item: &Item, value: &Value) {
        self.popup = Some(Popup::List(ListEdit {
            key: item.key.clone(),
            title: item.name.clone(),
            whole: item.element == "int",
            items: value.as_array().cloned().unwrap_or_default(),
            sel: 0,
            changed: false,
        }));
    }

    /// 窗里按了一个键：加、改开编辑窗，删、挪记下动过，`Esc` 收起时动过的整份存。
    pub(in crate::settings) fn list_key(
        &mut self,
        mut edit: ListEdit,
        key: KeyEvent,
        texts: &Texts,
    ) {
        match pairs::list_key(&mut edit.items, &mut edit.sel, key) {
            ListKey::Stay => self.popup = Some(Popup::List(edit)),
            ListKey::Changed => {
                edit.changed = true;
                self.popup = Some(Popup::List(edit));
            }
            ListKey::Edit(at) => {
                let now = at
                    .and_then(|i| edit.items.get(i))
                    .cloned()
                    .unwrap_or(Value::Null);
                let line = LineEdit::open(&edit.key, &edit.title, edit.whole, &now);
                self.popup = Some(Popup::Line(line.to(Target::ListItem(Box::new(edit), at))));
            }
            ListKey::Close if edit.changed => {
                self.write_setting(&edit.key, Value::Array(edit.items), texts);
            }
            ListKey::Close => self.popup = None,
        }
    }

    /// 编辑窗写好了：回到列表，换上、加上这一条（空的不加、改成空的去掉）；整数的写了别的，编辑窗里红字、不关。不是列表的
    /// 交回 `false`。
    pub(in crate::settings) fn list_write(
        &mut self,
        target: &Target,
        value: &Value,
        texts: &Texts,
    ) -> bool {
        let Target::ListItem(list, at) = target else {
            return false;
        };
        let mut list = (**list).clone();
        let empty = value.as_str().is_some_and(|s| s.is_empty());
        if list.whole && !empty && !value.is_i64() {
            let mut line = LineEdit::open(&list.key, &list.title, true, value);
            line.error = Some(texts.more.list.whole.clone());
            self.popup = Some(Popup::Line(line.to(target.clone())));
            return true;
        }
        match (*at, empty) {
            (None, true) => {}
            (None, false) => {
                list.items.push(value.clone());
                list.sel = list.items.len() - 1;
                list.changed = true;
            }
            (Some(i), true) if i < list.items.len() => {
                list.items.remove(i);
                list.sel = list.sel.min(list.items.len().saturating_sub(1));
                list.changed = true;
            }
            (Some(i), false) if list.items.get(i) != Some(value) && i < list.items.len() => {
                list.items[i] = value.clone();
                list.changed = true;
            }
            _ => {}
        }
        self.popup = Some(Popup::List(list));
        true
    }

    /// 密钥的列表这类开不了窗的：说一声。
    pub(in crate::settings) fn list_not_editable(&mut self, texts: &Texts) {
        self.say(texts.more.no_editor.clone(), Tone::Note);
    }
}

#[cfg(test)]
mod tests;
