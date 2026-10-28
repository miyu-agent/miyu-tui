//! 配置页的「开机启动」（蓝图 `tui.md`「配置页」第 49 条，核心 Z-1 下）：通用页最后一组「启动」一行开关。它不是配置项，开没
//! 开着是系统服务管理器那边的事：开配置页时问一次（`miyu service status`），`Enter` 当场装上、拿掉，不走草稿、不发
//! `config.set`。命令在后台跑（`crate::service`），跑完 App 交回来（[`Settings::service_replied`]）。

use serde::Deserialize;
use serde_json::Value;

use super::pages::schema::{GENERAL, Group, Item, Schema};
use super::{Settings, Texts, Tone};
use crate::service::{Reply, Verb};

/// 这一行的键：只在终端里，不发给核心。
pub const KEY: &str = "tui.service";

/// 这一组、这一行的字和状态行的话。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Words {
    /// 组名。
    pub group: String,
    /// 这一行。
    pub name: String,
    /// 正在装上、拿掉。
    pub enabling: String,
    pub disabling: String,
    /// 装上了（`{file}`）、拿掉了。
    pub enabled: String,
    pub disabled: String,
    /// 没成：`{step}`、`{said}`。
    pub failed: String,
    /// 不知道家目录、Windows 上设了数据根（核心的 `no_home`、`data_root_on_windows`）。
    pub no_home: String,
    pub windows: String,
}

/// 清单读来以后加上这一行：通用页最后一组。通用页都没有的（核心旧）、不知道核心程序在哪的（`known` 是假）不加。
pub fn add_row(schema: &mut Schema, words: &Words, known: bool) {
    if !known || schema.items.iter().any(|i| i.key == KEY) {
        return;
    }
    let Some(page) = schema
        .groups
        .iter()
        .find(|g| g.page == GENERAL)
        .map(|g| g.page.clone())
    else {
        return;
    };
    let at = schema
        .groups
        .iter()
        .rposition(|g| g.page == page)
        .map_or(schema.groups.len(), |i| i + 1);
    schema.groups.insert(
        at,
        Group {
            id: KEY.to_string(),
            name: words.group.clone(),
            page: page.clone(),
        },
    );
    schema.items.push(Item {
        key: KEY.to_string(),
        control: "toggle".to_string(),
        kind: "bool".to_string(),
        element: String::new(),
        suggestions: Vec::new(),
        options: Vec::new(),
        layers: vec!["system".to_string()],
        default: Value::Bool(false),
        name: words.name.clone(),
        page,
        group: KEY.to_string(),
        package: String::new(),
        common: false,
        unlisted: false,
    });
}

impl Settings {
    /// `Enter`：开着的拿掉、关着的装上（正跑着的排着，`crate::service`）。
    pub(super) fn toggle_service(&mut self, on: bool, texts: &Texts) {
        let words = &texts.more.service;
        let (verb, text) = if on {
            (Verb::Disable, &words.disabling)
        } else {
            (Verb::Enable, &words.enabling)
        };
        self.more.service_runs.push(verb);
        self.say(text.clone(), Tone::Busy);
    }

    /// 要在后台跑的（App 拿去跑）。
    pub fn take_service_runs(&mut self) -> Vec<Verb> {
        std::mem::take(&mut self.more.service_runs)
    }

    /// 跑完了：开关照装没装画；装上、拿掉的状态行说一声，没成的说哪一步、系统的原话。问状态没成的（核心旧）不说。
    pub fn service_replied(&mut self, reply: &Reply, texts: &Texts) {
        let words = &texts.more.service;
        match reply {
            Reply::Done {
                verb,
                installed,
                file,
                ..
            } => {
                if let Some(data) = self.data.as_mut() {
                    let origin = if *installed { "system" } else { "default" };
                    data.values.insert(
                        KEY.to_string(),
                        (Value::Bool(*installed), origin.to_string()),
                    );
                }
                self.refresh();
                match verb {
                    Verb::Enable => self.say(words.enabled.replace("{file}", file), Tone::Good),
                    Verb::Disable => self.say(words.disabled.clone(), Tone::Good),
                    Verb::Status => {}
                }
            }
            Reply::Failed {
                verb: Verb::Status, ..
            } => {}
            Reply::Failed {
                code, step, said, ..
            } => {
                let text = match code.as_str() {
                    "no_home" => words.no_home.clone(),
                    "data_root_on_windows" => words.windows.clone(),
                    _ => words.failed.replace("{step}", step).replace("{said}", said),
                };
                self.say(text, Tone::Bad);
            }
        }
    }
}

#[cfg(test)]
mod tests;
