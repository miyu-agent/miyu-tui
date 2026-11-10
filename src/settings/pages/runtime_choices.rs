//! 选项由包在运行时给的文字项（蓝图「配置页」第 43a 条的子条目，核心 SG-2 的 `choices_from`）：`Enter` 时问一次包
//! （`package.call`），回来了开下拉项的那个选择窗；问不到、回的不是选项的退回填一行字的编辑窗。

use serde_json::{Value, json};

use super::choose::{Choice, Choose, LineEdit};
use super::schema::Item;
use super::values::current;
use crate::core::Refusal;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Waiting};

/// 包回的 `{"choices": [{"value", "name"?, "description"?}]}` 读成选择窗的一行行：`value` 不是字的那一条不要；一条都
/// 没有的是 `None`（退回文字框）。`stored` 是现在存着的值：不在列表里的补在最后一行，照值写，能留着。
pub(super) fn read(got: &Value, stored: &Value) -> Option<Vec<Choice>> {
    let mut list: Vec<Choice> = got["choices"]
        .as_array()?
        .iter()
        .filter_map(|c| {
            let value = c["value"].as_str()?;
            let name = c["name"].as_str().filter(|n| !n.trim().is_empty());
            Some(Choice {
                value: Value::String(value.to_string()),
                label: crate::tabs::printable(name.unwrap_or(value)).into_owned(),
                note: c["description"]
                    .as_str()
                    .filter(|d| !d.trim().is_empty())
                    .map(|d| crate::tabs::printable(d).into_owned()),
                off: None,
            })
        })
        .collect();
    if list.is_empty() {
        return None;
    }
    if let Some(text) = stored.as_str().filter(|s| !s.is_empty())
        && !list.iter().any(|c| &c.value == stored)
    {
        list.push(Choice {
            value: stored.clone(),
            label: text.to_string(),
            note: None,
            off: None,
        });
    }
    Some(list)
}

impl Settings {
    /// `Enter` 这一项：问包要选项。已经在问的不重复问。
    pub(in crate::settings) fn ask_choices(&mut self, item: &Item) {
        let Some((package, method)) = &item.choices_from else {
            return;
        };
        if self
            .waiting
            .values()
            .any(|w| matches!(w, Waiting::Choices(_)))
        {
            return;
        }
        self.ask(
            "package.call",
            json!({"package": package, "method": method}),
            Waiting::Choices(item.key.clone()),
        );
    }

    /// 包答了：是选项的开选择窗，停在存着的值上；拒了的、不是选项的开编辑窗。这中间人开了别的窗、清单里没有这一项了
    /// 的不管。
    pub(in crate::settings) fn choices_answer(
        &mut self,
        key: &str,
        result: Result<Value, Refusal>,
    ) {
        if self.popup.is_some() {
            return;
        }
        let item = self
            .more
            .schema
            .as_ref()
            .and_then(|s| s.items.iter().find(|i| i.key == key))
            .cloned();
        let (Some(item), Some(data)) = (item, self.data.as_ref()) else {
            return;
        };
        let value = current(&item, data).0.clone();
        let list = result.ok().and_then(|got| read(&got, &value));
        self.popup = Some(match list {
            Some(list) => Popup::Choose(Choose::open(&item.key, &item.name, list, &value)),
            None => Popup::Line(LineEdit::open(&item.key, &item.name, false, &value)),
        });
    }
}

#[cfg(test)]
mod tests;
