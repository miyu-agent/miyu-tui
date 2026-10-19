//! 会话中途换人格、预设（蓝图 `tui.md`「新会话：人格、工作区」第 2a 条，2026-10-19 项目主人定放开，核心 P-13）：照推来的
//! `session.policy_changed` 换掉会话用的那一个（侧边栏、头像、跟随头像的配色跟着走），正文记一行暗色的「已换成」，名字照
//! 人格、预设列表（列表还没到的写编号）。之后核心开的那一轮压缩照手动压缩画。

use std::collections::BTreeMap;
use std::sync::{PoisonError, RwLock};

use super::{Kind, Transcript};
use crate::config::Texts;

/// 编号 → 名字：人格、预设两张。整个界面一份（会话换来换去、补读更早的一页都照它），App 拿到列表时换。
static NAMES: RwLock<(BTreeMap<String, String>, BTreeMap<String, String>)> =
    RwLock::new((BTreeMap::new(), BTreeMap::new()));

/// 拿到了人格（`persona` 是真）或者预设的列表：编号 → 名字整张换掉。
pub fn set_names(persona: bool, names: BTreeMap<String, String>) {
    let mut all = NAMES.write().unwrap_or_else(PoisonError::into_inner);
    if persona {
        all.0 = names;
    } else {
        all.1 = names;
    }
}

/// 照编号找名字；列表还没到、没有的写编号。
fn name(persona: bool, id: &str) -> String {
    let all = NAMES.read().unwrap_or_else(PoisonError::into_inner);
    let table = if persona { &all.0 } else { &all.1 };
    table.get(id).cloned().unwrap_or_else(|| id.to_string())
}

impl Transcript {
    /// 换了人格：空的是换成无人格。
    pub(super) fn persona_set(&mut self, id: String, texts: &Texts) {
        let words = &texts.persona_box;
        let name = if id.is_empty() {
            words.none.clone()
        } else {
            name(true, &id)
        };
        self.persona = (!id.is_empty()).then_some(id);
        self.note(Kind::Note, words.switched.replace("{name}", &name));
    }

    /// 换了预设。
    pub(super) fn preset_set(&mut self, id: String, texts: &Texts) {
        let name = name(false, &id);
        self.preset = Some(id);
        let line = texts.persona_box.preset_switched.replace("{name}", &name);
        self.note(Kind::Note, line);
    }
}
