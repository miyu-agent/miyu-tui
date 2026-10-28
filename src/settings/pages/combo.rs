//! 下拉加「其他」手写（蓝图「配置页」第 44 条，`control` 是 `combo`，核心 P-9 补：「用户」页的「回复语言」）：选择窗第一行
//! 「不设」，中间是 `config.schema` 这一项的建议值，最后一行「其他…」开编辑窗自己写。写进配置的就是那个字。

use serde_json::{Value, json};

use super::choose::{Choice, Choose, LineEdit};
use super::schema::Item;
use super::target::Target;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts};

/// 「其他…」那一行的值：配置里不会有这样的值。
pub fn other() -> Value {
    json!({"combo_other": true})
}

/// 选择窗的几行：「不设」（`words[0]`）、建议值、「其他…」（`words[1]`）。配置页和引导的「关于你」那一步共用。
pub fn rows(suggestions: &[String], words: [&str; 2]) -> Vec<Choice> {
    let row = |value: Value, label: &str| Choice {
        value,
        label: label.to_string(),
        note: None,
        off: None,
    };
    let mut list = vec![row(Value::Null, words[0])];
    list.extend(suggestions.iter().map(|s| row(Value::String(s.clone()), s)));
    list.push(row(other(), words[1]));
    list
}

/// 选择窗停在哪：没设的停在「不设」，在建议里的停在它，别的停在「其他…」。
pub fn landing(suggestions: &[String], value: &Value) -> Value {
    match value {
        Value::String(s) if !suggestions.contains(s) => other(),
        Value::String(_) => value.clone(),
        _ => Value::Null,
    }
}

/// 配置页这一项的选择窗的几行。
fn choices(item: &Item, texts: &Texts) -> Vec<Choice> {
    let [unset, other_label] = &texts.more.combo;
    rows(&item.suggestions, [unset, other_label])
}

/// 配置页这一项的选择窗停在哪。
fn at(item: &Item, value: &Value) -> Value {
    landing(&item.suggestions, value)
}

impl Settings {
    /// 开选择窗（`item` 现在的值是 `value`）。
    pub(in crate::settings) fn open_combo(&mut self, item: &Item, value: &Value, texts: &Texts) {
        let mut choose = Choose::open(
            &item.key,
            &item.name,
            choices(item, texts),
            &at(item, value),
        );
        choose.target = Target::Combo;
        self.popup = Some(Popup::Choose(choose));
    }

    /// 选择窗选好了：「其他…」开编辑窗（原来写的不在建议里的留着），交回 `true`；别的交回 `false`，照旧写系统配置（「不设」
    /// 是空的，去掉这一项）。
    pub(in crate::settings) fn combo_write(
        &mut self,
        target: &Target,
        key: &str,
        value: &Value,
    ) -> bool {
        if *target != Target::Combo || *value != other() {
            return false;
        }
        let item = self
            .more
            .schema
            .as_ref()
            .and_then(|s| s.items.iter().find(|i| i.key == key))
            .cloned();
        let (Some(item), Some(data)) = (item, self.data.as_ref()) else {
            return true;
        };
        let (now, _) = super::current(&item, data);
        let written = match now {
            Value::String(s) if !item.suggestions.contains(s) => now.clone(),
            _ => Value::Null,
        };
        self.popup = Some(Popup::Line(LineEdit::open(
            key, &item.name, false, &written,
        )));
        true
    }
}

#[cfg(test)]
mod tests;
