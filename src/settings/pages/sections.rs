//! 软件包的设置分组（蓝图「配置页」第 43d 条，2026-11-10 项目主人：「这软件包设置页，怎么是纯平铺，而且选择 mimo 也会出现
//! minimax 的配置项目」「可以看看 im web 的会话卡片的设计」；核心格式 core1 定的，排成核心 SG-1）：`config.schema` 里包的
//! 每一项带 `section`（组编号，没分组的空着），包那一组带 `sections`（`[{id, name, shown_when?: {key, is: [...]}}]`）。
//! 终端照它在包那一页分块：没分组的照旧排在最前、不带标题；一组一块，组名做标题，后面暗色写「默认」或「已改」；
//! `shown_when` 不满足的组不画（值照旧留着，核心照旧读、照旧校验），一项不剩的组不画。旧核心不带这两格的照原来平铺。
//! 一组在 [`Schema::groups`] 里是一个组，编号是 `<包>#<组>`，排在包那一组后面；组里的项的 `group` 改成它，`package` 记上包。

use std::collections::HashMap;

use serde_json::Value;

use super::schema::{Group, Row, Schema};
use crate::settings::data::Data;

/// 包里的一组：在 [`Schema::groups`] 里的编号、属于哪个包、什么时候画。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// 在 [`Schema::groups`] 里的编号（`<包>#<组>`）。
    pub group: String,
    /// 属于哪个包。
    pub package: String,
    /// 出现的条件：键（全名）、值是这几个里的一个才画；没写的一直画。
    pub when: Option<(String, Vec<Value>)>,
}

/// 分组用到的字（`text/*.json` 的 `settings.more.sections`）。
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionTexts {
    /// 组名后面写的：默认、已改。
    pub marks: [String; 2],
    /// 光标停在改过的组里时的按键提示（多一个 `D` 恢复这一组）。
    pub hints: Vec<[String; 2]>,
    /// `D` 问的那一句：标题 `{name}`、一行话；按钮同恢复一项的。
    pub unset: [String; 2],
}

/// 包的编号和组编号中间的记号。
const SEP: char = '#';

/// 读 `config.schema` 的回应里包的分组：照 `sections` 在包那一组后面插组、把带 `section` 的项挪进去。包没写 `sections` 的、
/// 项的 `section` 不在它写的组里的照旧留在包那一组。
pub fn split(schema: &mut Schema, got: &Value) {
    let marked: HashMap<&str, &str> = got["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|i| Some((i["key"].as_str()?, i["section"].as_str()?)))
        .filter(|(_, s)| !s.is_empty())
        .collect();
    for raw in got["groups"].as_array().into_iter().flatten() {
        let Some(list) = raw["sections"].as_array().filter(|l| !l.is_empty()) else {
            continue;
        };
        let package = raw["id"].as_str().unwrap_or_default().to_string();
        let Some(at) = schema.groups.iter().position(|g| g.id == package) else {
            continue;
        };
        let page = schema.groups[at].page.clone();
        let mut added = 0;
        for s in list {
            let Some(id) = s["id"].as_str().filter(|id| !id.is_empty()) else {
                continue;
            };
            let group = format!("{package}{SEP}{id}");
            let when = s["shown_when"]["key"].as_str().map(|key| {
                let is = s["shown_when"]["is"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                (key.to_string(), is)
            });
            for item in schema.items.iter_mut().filter(|i| i.group == package) {
                if marked.get(item.key.as_str()) == Some(&id) {
                    if item.package.is_empty() {
                        item.package.clone_from(&package);
                    }
                    item.group.clone_from(&group);
                }
            }
            let name = name_of(&s["name"]).unwrap_or(id).to_string();
            schema.groups.insert(
                at + 1 + added,
                Group {
                    id: group.clone(),
                    name,
                    page: page.clone(),
                    description: None,
                },
            );
            added += 1;
            schema.sections.push(Section {
                group,
                package: package.clone(),
                when,
            });
        }
    }
}

/// 组名：核心照连接的语言交一串字；交成按语言分的表的，挑第一个不空的。
fn name_of(v: &Value) -> Option<&str> {
    v.as_str()
        .or_else(|| v.as_object()?.values().find_map(Value::as_str))
        .filter(|s| !s.is_empty())
}

impl Schema {
    /// 第 `g` 组是包里的一组：交回它。
    pub fn section(&self, g: usize) -> Option<&Section> {
        let id = &self.groups.get(g)?.id;
        self.sections.iter().find(|s| &s.group == id)
    }

    /// 第 `g` 组这一刻藏着：是包里的一组，写了条件、那一项现在的值（系统配置写了的照写的，没写的照清单的默认）不在里面。
    /// 读来的值还没到的、条件的键清单里找不到的照画（core1 定）。
    pub fn hidden(&self, g: usize, data: Option<&Data>) -> bool {
        let (Some((key, is)), Some(data)) = (self.section(g).and_then(|s| s.when.as_ref()), data)
        else {
            return false;
        };
        let now = match data.values.get(key) {
            Some((value, _)) => value,
            None => match self.items.iter().find(|i| &i.key == key) {
                Some(item) => &item.default,
                None => return false,
            },
        };
        !is.contains(now)
    }

    /// 第 `g` 组改过：组里有一项写在系统配置里。
    pub fn changed(&self, g: usize, data: &Data) -> bool {
        self.keys_of(g).iter().any(|k| data.system.contains_key(*k))
    }

    /// 第 `g` 组里的项的键，照登记的先后。
    pub fn keys_of(&self, g: usize) -> Vec<&str> {
        let Some(group) = self.groups.get(g) else {
            return Vec::new();
        };
        self.items
            .iter()
            .filter(|i| i.group == group.id)
            .map(|i| i.key.as_str())
            .collect()
    }

    /// 第 `i` 项在哪一组（包里的一组才交）。
    pub fn section_of_item(&self, i: usize) -> Option<usize> {
        let group = &self.items.get(i)?.group;
        let g = self.groups.iter().position(|g| &g.id == group)?;
        self.section(g).map(|_| g)
    }
}

impl crate::settings::Settings {
    /// `D`：光标停在包里改过的一组里，问一句再把这一组写在系统配置里的几项都去掉（「配置页」第 43d 条，同 im 网页点「已改」）；
    /// 不在组里的、组里都是默认的说一句没设。
    pub(in crate::settings) fn ask_unset_section(
        &mut self,
        page: &str,
        at: usize,
        texts: &crate::settings::Texts,
    ) {
        let (Some(schema), Some(data)) = (self.more.schema.as_ref(), self.data.as_ref()) else {
            return;
        };
        let words = &texts.more;
        let Some(g) = self
            .page_items(page)
            .get(at)
            .and_then(|&i| schema.section_of_item(i))
        else {
            self.say(words.nothing_to_unset.clone(), crate::settings::Tone::Note);
            return;
        };
        let keys: Vec<String> = schema
            .keys_of(g)
            .into_iter()
            .filter(|k| data.system.contains_key(*k))
            .map(str::to_string)
            .collect();
        if keys.is_empty() {
            self.say(words.nothing_to_unset.clone(), crate::settings::Tone::Note);
            return;
        }
        let name = &schema.groups[g].name;
        self.popup = Some(crate::settings::popup::Popup::Confirm(
            crate::settings::popup::Confirm {
                title: words.sections.unset[0].replace("{name}", name),
                lines: vec![(words.sections.unset[1].clone(), true)],
                buttons: vec![
                    (words.unset[2].clone(), true),
                    (words.unset[3].clone(), false),
                ],
                sel: 1,
                ask: crate::settings::popup::Delete::Settings(keys),
            },
        ));
    }
}

/// 去掉藏着的组：组名那一行和它下面的项都不要，到下一个组名为止。
pub fn visible(rows: Vec<Row>, schema: &Schema, data: Option<&Data>) -> Vec<Row> {
    let mut hiding = false;
    rows.into_iter()
        .filter(|row| {
            if let Row::Group(g) = row {
                hiding = schema.hidden(*g, data);
            }
            !hiding
        })
        .collect()
}

#[cfg(test)]
mod tests;
