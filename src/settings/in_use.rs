//! 改开着的会话正在用的人格、预设（蓝图 `tui.md`「新会话：人格、工作区」第 2b 条，2026-10-20 项目主人定）：改了下一轮
//! 缓存全部失效。从 `/config` 进来改的存之前先问一句（「是」存、「否」不存，回到编辑，填的不丢）；从人格框、预设框按
//! `e` 进来的（`lone`）不问、存成了记下，关了详情窗由 App 问要不要压缩。还没开会话的都不问。

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use super::pages::persona_edit::PromptEdit;
use super::popup::{Confirm, Delete, Popup};
use super::{Outcome, Settings, Texts, Waiting};

/// 改正在用的那一句的字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InUseTexts {
    /// 改了正在用的人格（项目主人原话，人格版）。
    pub persona: String,
    /// 改了正在用的预设（项目主人原话）。
    pub preset: String,
    /// 两个按钮：是、否。
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

/// 盯着正在用的那两样。
#[derive(Debug, Default)]
pub struct Watch {
    /// 开着的会话现在用的人格（无人格是空的编号）；还没开会话的是 `None`。
    pub persona: Option<String>,
    /// 现在用的预设；还没开会话的是 `None`。
    pub preset: Option<String>,
    /// 从框按 `e` 进来的：只开这一项的详情窗，关了回框；存的时候不问。
    pub lone: bool,
    /// 存成了的（人格、预设的编号）：App 关了详情窗照它问要不要压缩。
    pub saved: Vec<(Item, String)>,
    /// 发出去还没回来的存：编号到存的是哪一样。
    sending: HashMap<u64, (Item, String)>,
    /// 等人回答的那一次存。
    held: Option<Held>,
    /// 这一下按键以前开着的窗：「否」回到它。
    before: Option<Popup>,
    /// 提示词交给编辑器改完的、改过的字：「否」照改过的字再交给编辑器。
    pub(super) reopen: Option<(PromptEdit, String)>,
    /// 再交给编辑器的字（[`Settings::take_prompt`] 先拿它）。
    pub(super) draft: Option<String>,
}

/// 等人回答的一次存：照原样发的方法、参数、等的回应，「是」「否」各回到哪个窗。
#[derive(Debug)]
struct Held {
    method: &'static str,
    params: Value,
    waiting: Waiting,
    item: Item,
    id: String,
    back: Option<Popup>,
    after: Option<Popup>,
    reopen: Option<(PromptEdit, String)>,
    asked: bool,
}

/// `persona.set`、`preset.set` 改的是哪一样（新建的、只换头像这几样外观的是 `None`：外观不进给模型的字）。
fn changes(method: &str, params: &Value) -> Option<(Item, String)> {
    let (item, key) = match method {
        "persona.set" if params.get("changes").is_some() || params.get("prompts").is_some() => {
            (Item::Persona, "persona")
        }
        "preset.set" => (Item::Preset, "preset"),
        _ => return None,
    };
    params[key].as_str().map(|id| (item, id.to_string()))
}

impl Settings {
    /// 从人格框、预设框按 `e` 进来：直接开那一项的详情窗（第 2b 条）。
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

    /// 发一个请求以前：改正在用的、从 `/config` 进来的先扣下等人回答；别的照发。
    pub(super) fn ask(&mut self, method: &'static str, params: Value, waiting: Waiting) {
        let changing = changes(method, &params);
        let in_use = changing.as_ref().filter(|(item, id)| {
            let current = match item {
                Item::Persona => &self.watch.persona,
                Item::Preset => &self.watch.preset,
            };
            current.as_deref() == Some(id.as_str())
        });
        if let Some((item, id)) = in_use.cloned()
            && !self.watch.lone
            && self.watch.held.is_none()
        {
            self.watch.held = Some(Held {
                method,
                params,
                waiting,
                item,
                id,
                back: self.watch.before.take(),
                after: None,
                reopen: self.watch.reopen.take(),
                asked: false,
            });
            return;
        }
        self.watch.reopen = None;
        self.post(method, params, waiting);
        if let Some(changing) = changing {
            self.watch.sending.insert(self.next, changing);
        }
    }

    /// 照发。
    fn post(&mut self, method: &'static str, params: Value, waiting: Waiting) {
        self.next += 1;
        self.waiting.insert(self.next, waiting);
        self.asks.push((self.next, method, params));
    }

    /// 回来了：存成了的记下（App 照它问要不要压缩）。
    pub(super) fn note_saved(&mut self, tag: u64, ok: bool) {
        if let Some(saved) = self.watch.sending.remove(&tag).filter(|_| ok) {
            self.watch.saved.push(saved);
        }
    }

    /// 按了一个键（鼠标点也走这里）：扣下了一次存的，开窗问。
    pub fn key(&mut self, key: ratatui::crossterm::event::KeyEvent, texts: &Texts) -> Outcome {
        let watching =
            !self.watch.lone && (self.watch.persona.is_some() || self.watch.preset.is_some());
        if watching && self.watch.held.is_none() {
            self.watch.before = self.popup.clone();
        }
        let outcome = self.press(key, texts);
        self.watch.before = None;
        self.raise_held(texts);
        outcome
    }

    /// 扣下了一次存、还没问的：开窗问（字照项目主人的原话）。
    pub fn raise_held(&mut self, texts: &Texts) {
        let Some(held) = self.watch.held.as_mut().filter(|h| !h.asked) else {
            return;
        };
        held.asked = true;
        held.after = self.popup.take();
        if held.back.is_none() {
            held.back = held.after.clone();
        }
        let words = &texts.in_use;
        let (line, list) = match held.item {
            Item::Persona => (&words.persona, &self.more.personas),
            Item::Preset => (&words.preset, &self.more.presets),
        };
        let name = list
            .iter()
            .flatten()
            .find(|p| p.id == held.id)
            .map_or(held.id.clone(), |p| p.label().to_string());
        self.popup = Some(Popup::Confirm(Confirm {
            title: name,
            lines: vec![(line.clone(), false)],
            buttons: words.buttons.iter().map(|b| (b.clone(), false)).collect(),
            sel: 0,
            ask: Delete::Held,
        }));
    }

    /// 人答了：「是」照原样存、回到存完的窗；「否」不存、回到编辑（交给编辑器改的再交给编辑器）。
    pub(super) fn settle_held(&mut self, yes: bool) -> Outcome {
        let Some(held) = self.watch.held.take() else {
            return Outcome::Stay;
        };
        if yes {
            self.popup = held.after;
            self.post(held.method, held.params, held.waiting);
            self.watch.sending.insert(self.next, (held.item, held.id));
        } else {
            self.popup = held.back;
            if let Some((edit, text)) = held.reopen {
                self.prompt = Some(edit);
                self.watch.draft = Some(text);
                self.editing = true;
            }
        }
        Outcome::Stay
    }
}

#[cfg(test)]
mod tests;
