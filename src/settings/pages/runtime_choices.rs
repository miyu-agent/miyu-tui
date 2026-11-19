//! 选项由包在运行时给的文字项（蓝图「配置页」第 43a 条的子条目，核心 SG-2 的 `choices_from`）：`Enter` 时问一次包
//! （`package.call`），回来了开下拉项的那个选择窗；问不到、回的不是选项的退回填一行字的编辑窗。读到清单时也问一次，那一行照
//! 选项的名字写（「引擎 DeepSeek 功能全家桶」，不写存着的原值 `deepseek-tools`），问不到的照原值写。值是空的那个选项就是「不设」
//! 时用的（语音输入的本机 SenseVoice）：没设时写它的名字，选它是去掉这一项。

use serde_json::{Value, json};

use super::choose::{Choice, Choose, LineEdit};
use super::schema::Item;
use super::values::current;
use crate::core::Refusal;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Waiting};

/// 包回的 `{"choices": [{"value", "name"?, "description"?}]}` 读成选择窗的一行行：`value` 不是字的那一条不要；一条都
/// 没有的是 `None`（退回文字框）。值是空的那一条当「不设」（`null`，选了去掉这一项）。`stored` 是现在存着的值：不在列表里的
/// 补在最后一行，照值写，能留着。
pub(super) fn read(got: &Value, stored: &Value) -> Option<Vec<Choice>> {
    let mut list: Vec<Choice> = got["choices"]
        .as_array()?
        .iter()
        .filter_map(|c| {
            let value = c["value"].as_str()?;
            let name = c["name"].as_str().filter(|n| !n.trim().is_empty());
            Some(Choice {
                value: match value {
                    "" => Value::Null,
                    value => Value::String(value.to_string()),
                },
                label: crate::tabs::printable(name.unwrap_or(value)).into_owned(),
                note: None,
                off: None,
                // 说明写在名字下面（「配置页」第 33 条「选项的说明和提示」）。
                description: c["description"]
                    .as_str()
                    .filter(|d| !d.trim().is_empty())
                    .map(|d| crate::tabs::printable(d).into_owned()),
                notice: None,
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
            description: None,
            notice: None,
        });
    }
    Some(list)
}

/// 存着的值照选项写成名字：没设的、空的字照「不设」那一条（值是空的）；没有对得上的是 `None`（照原值写）。
pub fn name_of(list: &[Choice], value: &Value) -> Option<String> {
    let value = match value {
        Value::String(text) if text.is_empty() => &Value::Null,
        value => value,
    };
    list.iter()
        .find(|c| &c.value == value)
        .map(|c| c.label.clone())
}

impl super::More {
    /// 带 `choices_from` 的一项那一行写什么：问到过选项的照名字写；没问到过、对不上的是 `None`。
    pub fn choice_name(&self, item: &Item, value: &Value) -> Option<String> {
        item.choices_from.as_ref()?;
        name_of(self.package_choices.get(&item.key)?, value)
    }
}

impl Settings {
    /// 读到清单了：带 `choices_from` 的每一项问一次包，那一行照选项的名字写。
    pub(in crate::settings) fn ask_choice_names(&mut self) {
        let asks: Vec<(String, String, String)> = self
            .more
            .schema
            .iter()
            .flat_map(|s| s.items.iter())
            .filter_map(|i| {
                let (package, method) = i.choices_from.clone()?;
                Some((i.key.clone(), package, method))
            })
            .collect();
        for (key, package, method) in asks {
            let params = json!({"package": package, "method": method});
            self.ask("package.call", params, Waiting::ChoiceNames(key));
        }
    }

    /// 记下包给一项的选项（那一行照它写名字）；问不到的去掉，照原值写。
    pub(in crate::settings) fn note_choices(&mut self, key: &str, result: Option<&Value>) {
        let stored = Value::Null;
        match result.and_then(|got| read(got, &stored)) {
            Some(list) => self.more.package_choices.insert(key.to_string(), list),
            None => self.more.package_choices.remove(key),
        };
    }

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
        // 存着空的字同没设：停在「不设」那一条（值是空的选项）上。
        let value = match current(&item, data).0 {
            Value::String(text) if text.is_empty() => Value::Null,
            value => value.clone(),
        };
        self.note_choices(key, result.as_ref().ok());
        let list = result.ok().and_then(|got| read(&got, &value));
        self.popup = Some(match list {
            Some(list) => Popup::Choose(Choose::open(&item.key, &item.name, list, &value)),
            None => Popup::Line(LineEdit::open(&item.key, &item.name, false, &value)),
        });
    }
}

#[cfg(test)]
mod tests;
