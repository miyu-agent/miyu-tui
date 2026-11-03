//! 一项的值怎么读、怎么写给人看、下拉里列什么（蓝图「配置页」第 33 到 35、47 条）：照清单画的一页和各个窗共用。

use serde_json::Value;

use super::choose::Choice;
use super::schema::Item;
use super::{DEFAULT_PERSONA, DEFAULT_PRESET, LOOK, MASCOT, Texts, list, secret};
use crate::core::Persona;
use crate::settings::data::Data;

/// 一项现在的值：读来的最终值和来自哪一层；哪一层都没写的照清单的默认，来自「默认」。
pub fn current<'a>(item: &'a Item, data: &'a Data) -> (&'a Value, &'a str) {
    match data.values.get(&item.key) {
        Some((value, origin)) => (value, origin.as_str()),
        None => (&item.default, "default"),
    }
}

/// 名字照列表写、下拉照列表列的项（默认人格、默认预设）：不管清单写什么控件，一律开选择窗。
pub fn is_named(key: &str) -> bool {
    key == DEFAULT_PERSONA || key == DEFAULT_PRESET || key == MASCOT || key == LOOK
}

/// 值写成给人看的样子：下拉写选项的名字（默认人格、默认预设写名字，`named` 是它们的列表），开关写开、关，密钥写设没设，
/// 列表用顿号连（私密目录两项只写个数）。
pub fn shown(item: &Item, value: &Value, named: Option<&[Persona]>, texts: &Texts) -> String {
    // 记录列表写各条的标题（第 47b 条）。
    if let Some(titles) = super::records::summary(item, value, &texts.records) {
        return titles;
    }
    if let (Some(list), Some(id)) = (named, value.as_str())
        && let Some(p) = list.iter().find(|p| p.id == id)
    {
        return p.label().to_string();
    }
    // 选着的吉祥物包、主题包卸掉了：编号后面写没装（「吉祥物包」第 3 条、「主题」第 5 条）。
    if (item.key == MASCOT || item.key == LOOK)
        && named.is_some()
        && let Some(id) = value.as_str()
    {
        return texts.mascot_missing.replace("{id}", id);
    }
    if let Some(name) = own_choice(item, value, texts) {
        return name;
    }
    if let Some(name) = item
        .options
        .iter()
        .find(|o| o.value == *value)
        .map(|o| &o.name)
    {
        return name.clone();
    }
    // 密钥：只写设了没设，不露引用（第 48 条）。
    if secret::is_secret(item) {
        return texts.secret[usize::from(value.is_null())].clone();
    }
    if let Some(n) = list::counted(item, value) {
        return match n {
            0 => "—".to_string(),
            n => texts.counted.replace("{n}", &n.to_string()),
        };
    }
    match (item.control.as_str(), value) {
        // 默认人格没设：就是无人格。
        (_, Value::Null) if item.key == DEFAULT_PERSONA => texts.no_persona.clone(),
        // 吉祥物没设：就是内置的。
        (_, Value::Null) if item.key == MASCOT => texts.builtin_mascot.clone(),
        // 主题没设：就是默认主题。
        (_, Value::Null) if item.key == LOOK => texts.default_look.clone(),
        // 下拉加「其他」手写没设：写「不设」（「配置页」第 44 条）。
        ("combo", Value::Null) => texts.combo[0].clone(),
        (_, Value::Null) => "—".to_string(),
        // 几行字：只写第一行，多行的末尾 `…`（第 45 条）。
        ("textarea", Value::String(s)) if s.trim_end().contains('\n') => {
            format!(
                "{}{}",
                s.lines().next().unwrap_or_default(),
                crate::sheet::ellipsis()
            )
        }
        ("toggle", Value::Bool(on)) => texts.toggle[usize::from(!*on)].clone(),
        (_, Value::String(s)) => s.clone(),
        // 空列表同没设（「配置页」第 47 条：删光了写的是空列表）。
        (_, Value::Array(list)) if list.is_empty() => "—".to_string(),
        (_, Value::Array(list)) => list
            .iter()
            .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_string))
            .collect::<Vec<_>>()
            .join("、"),
        (_, other) => other.to_string(),
    }
}

/// 下拉的选择窗里列什么：默认人格、默认预设照列表（`named`，写错的选不了），别的照清单的选项。
pub fn choices(item: &Item, named: Option<&[Persona]>, texts: &Texts) -> Vec<Choice> {
    if let Some(list) = named {
        // 默认人格第一行「无人格」：选了就不设（2026-10-08 项目主人：人格允许为空）。
        let first = match item.key.as_str() {
            DEFAULT_PERSONA => Some(&texts.no_persona),
            // 吉祥物第一行「内置」：选了就不设（「吉祥物包」第 3 条）。
            MASCOT => Some(&texts.builtin_mascot),
            // 主题第一行「默认主题」：选了就不设（「主题」第 5 条）。
            LOOK => Some(&texts.default_look),
            _ => None,
        };
        let none = first.map(|label| Choice {
            value: Value::Null,
            label: label.clone(),
            note: None,
            off: None,
        });
        let named = list.iter().map(|p| Choice {
            value: Value::from(p.id.clone()),
            label: p.label().to_string(),
            // 只写名字，说明在人格页、预设页看（2026-10-07 项目主人：没必要）。
            note: None,
            off: p.problem.clone(),
        });
        return none.into_iter().chain(named).collect();
    }
    item.options
        .iter()
        .map(|o| Choice {
            value: o.value.clone(),
            label: own_choice(item, &o.value, texts).unwrap_or_else(|| o.name.clone()),
            note: o.note.clone(),
            // 用不了的列着、灰的、选不了，为什么照头自己的一句（核心 R-5 三补）。
            off: (!o.available).then(|| texts.unavailable.clone()),
        })
        .collect()
}

/// 终端自己的选项（`tui.theme` 这类）照界面语言的名字；没登记的是 `None`，照清单写。
fn own_choice(item: &Item, value: &Value, texts: &Texts) -> Option<String> {
    texts
        .choice_names
        .get(&item.key)
        .and_then(|names| names.get(value.as_str()?))
        .cloned()
}
