//! 从人格框、预设框按 `e` 进来改（蓝图 `tui.md`「新会话：人格、工作区」第 2b 条，2026-10-20 项目主人定）：只开那一项的
//! 详情窗，关了回框；改成了的记下，App 照它问要不要压缩（改了正在用的，下一轮缓存全部失效）。从 `/config` 进来改的不问
//! （同一天项目主人：「配置页的就不弹提示了。没有人会特意进配置页改这东西的。」）。

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use super::{Settings, Waiting};

/// 问要不要压缩那一句的字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InUseTexts {
    /// 改了正在用的人格（项目主人原话，人格版）。
    pub persona: String,
    /// 改了正在用的预设（项目主人原话）。
    pub preset: String,
    /// 两行：是、否。
    pub buttons: [String; 2],
}

/// 人格还是预设。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// 人格。
    Persona,
    /// 预设。
    Preset,
}

/// 从框进来的那一趟。
#[derive(Debug, Default)]
pub struct Watch {
    /// 从框按 `e` 进来的：只开这一项的详情窗，关了回框。
    pub lone: bool,
    /// 改成了的（人格、预设的编号）：App 关了详情窗照它问要不要压缩。
    pub saved: Vec<(Item, String)>,
    /// 发出去还没回来的改：编号到改的是哪一样。
    sending: HashMap<u64, (Item, String)>,
}

/// 这个请求改不改给模型看的东西、改的是哪一样（和网页对齐，2026-10-20）：人格的 `changes`、`prompts`（名字、简单描述、子代理
/// 可选、提示词、示范对话），删掉、恢复出厂；预设改哪一项都算。新建的、只换头像、主色、背景的是 `None`。
fn changes(method: &str, params: &Value) -> Option<(Item, String)> {
    let (item, key) = match method {
        "persona.set" if params.get("changes").is_some() || params.get("prompts").is_some() => {
            (Item::Persona, "persona")
        }
        "persona.delete" => (Item::Persona, "persona"),
        "preset.set" | "preset.delete" => (Item::Preset, "preset"),
        _ => return None,
    };
    params[key].as_str().map(|id| (item, id.to_string()))
}

impl Settings {
    /// 从人格框、预设框按 `e` 进来：直接开那一项的详情窗。
    pub fn one(online: bool, item: Item, id: String) -> Self {
        let mut page = Self::open(false, online);
        page.on_menu = false;
        page.watch.lone = true;
        page.more.section = Some(match item {
            Item::Persona => super::pages::Section::Personas(0),
            Item::Preset => super::pages::Section::Presets(0),
        });
        match item {
            Item::Persona => page.show_persona(id),
            Item::Preset => page.show_preset(id),
        }
        page
    }

    /// 从框进来的详情窗关了：回框。
    pub fn lone_done(&self) -> bool {
        self.watch.lone && self.popup.is_none() && self.prompt.is_none() && !self.editing
    }

    /// 发一个请求；从框进来的、改了东西的记着等回来。
    pub(super) fn ask(&mut self, method: &'static str, params: Value, waiting: Waiting) {
        let changing = changes(method, &params).filter(|_| self.watch.lone);
        self.next += 1;
        self.waiting.insert(self.next, waiting);
        self.asks.push((self.next, method, params));
        if let Some(changing) = changing {
            self.watch.sending.insert(self.next, changing);
        }
    }

    /// 回来了：改成了的记下。
    pub(super) fn note_saved(&mut self, tag: u64, ok: bool) {
        if let Some(saved) = self.watch.sending.remove(&tag).filter(|_| ok) {
            self.watch.saved.push(saved);
        }
    }
}

#[cfg(test)]
mod tests;
