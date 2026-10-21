//! 给人看的字（工具的显示名、说法的模板）：核心 W-1 起向核心要（`human.get`），演示不再自己读核心的资源目录（蓝图
//! `tui.md`「界面语言」）。连上时、换语言时各要一次；没要到的（没连上、核心太旧）是空的，工具名照原名写。

use std::collections::BTreeMap;

use miyu_client::protocol::Said;
use miyu_client::protocol::Template;
use miyu_client::texts::{Face, clean};
use serde_json::Value;

/// MCP 服务的工具名的开头（核心 `miyu_tool::catalog::MCP_PREFIX`）。
const MCP: &str = "mcp__";

/// 一种语言的字。
#[derive(Debug, Clone, Default)]
pub struct Human {
    tools: BTreeMap<String, Face>,
    packages: BTreeMap<String, String>,
    icons: BTreeMap<String, String>,
    said: BTreeMap<String, Template>,
}

impl Human {
    /// 照 `human.get` 的回应造：`tools` 每件工具的样子，`packages` 每件工具属于哪个包（核心 X-24 补），`said` 每一句说法的
    /// 模板原文；读不懂的那一件、那一句不要。
    pub fn from_reply(reply: &Value) -> Human {
        let tools = reply["tools"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(name, face)| {
                Some((name.clone(), serde_json::from_value(face.clone()).ok()?))
            })
            .collect();
        let packages = reply["packages"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(name, package)| Some((name.clone(), package.as_str()?.to_string())))
            .collect();
        let said = reply["said"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(key, source)| {
                Some((key.clone(), Template::parse(source.as_str()?).ok()?))
            })
            .collect();
        Human {
            tools,
            packages,
            icons: BTreeMap::new(),
            said,
        }
    }

    /// 叫 `name` 的工具给人看的样子；没有的是空的。
    pub fn tool(&self, name: &str) -> Option<&Face> {
        self.tools.get(name)
    }

    /// 叫 `name` 的工具这一刻属于哪个包（工具目录里的包编号，核心 X-24）；不在目录里的是 `None`。
    pub fn package(&self, name: &str) -> Option<&str> {
        self.packages.get(name).map(String::as_str)
    }

    /// 换上每个包清单里写的图标（包编号到 Lucide 名，`package.list` 的 `icon`）：`human.get` 不带，另外要。
    pub fn set_icons(&mut self, icons: BTreeMap<String, String>) {
        self.icons = icons;
    }

    /// 每个包清单里写的图标（包编号到 Lucide 名）。
    pub fn icons(&self) -> &BTreeMap<String, String> {
        &self.icons
    }

    /// 叫 `name` 的工具的包在清单里写的图标（Lucide 名）；不属于哪个包、包没写的是 `None`。
    pub fn package_icon(&self, name: &str) -> Option<&str> {
        self.icons.get(self.package(name)?).map(String::as_str)
    }

    /// 叫 `name` 的工具给人看的名字：核心给了的照它；没给的照原名，MCP 服务的工具（`mcp__<服务>__<工具>`，核心 X-3）写
    /// 「服务 · 工具」（2026-10-11 项目主人定；核心的图纸写「头怎么显示归头」）。
    pub fn tool_name(&self, name: &str) -> String {
        if let Some(face) = self.tool(name) {
            return face.name.clone();
        }
        name.strip_prefix(MCP)
            .and_then(|rest| rest.split_once("__"))
            .filter(|(service, tool)| !service.is_empty() && !tool.is_empty())
            .map_or_else(
                || name.to_string(),
                |(service, tool)| format!("{service}{}{tool}", crate::sheet::sep()),
            )
    }

    /// 一步给人看的名字：调用那一刻记下的显示名（`display`，语言到名字）照 `lang` 挑，挑不到照 en、zh、ja 的先后（核心
    /// 9-8 六补）；都没有的照这一刻的工具表（[`Human::tool_name`]）。
    pub fn step_name(&self, name: &str, display: &BTreeMap<String, String>, lang: &str) -> String {
        [lang, "en", "zh", "ja"]
            .iter()
            .find_map(|l| display.get(*l))
            .cloned()
            .unwrap_or_else(|| self.tool_name(name))
    }

    /// 照说法换成一句话，字段先过一遍核心那份 `clean`。没有这一句、或者少了字段的，是空的。
    pub fn say(&self, said: &Said) -> Option<String> {
        let template = self.said.get(&said.key)?;
        let fields: BTreeMap<&str, &str> = said
            .fields
            .iter()
            .map(|(field, value)| (field.as_str(), value.as_str()))
            .collect();
        template.fill(&fields, clean).ok()
    }
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
