//! 管理员模式下配置页写哪一层（蓝图 `tui.md`「管理员模式」第 3 条，2026-10-24 项目主人定）：开着写系统那一层（大家
//! 共用），关着写个人设置；只能写系统的项，关着时锁着。存 key 跟着模式走：`secret.set` 不写 `layer`，带不带 `scope` 由
//! 连核心的那一头照模式定（`core/scope.rs`）。

use super::pages::schema::Item;
use super::{Settings, Texts};

impl Settings {
    /// 管理员模式开着。
    pub fn admin_on(&self) -> bool {
        self.admin == Some(true)
    }

    /// 清单里的这一项。
    fn item(&self, key: &str) -> Option<&Item> {
        self.more
            .schema
            .as_ref()
            .and_then(|s| s.items.iter().find(|i| i.key == key))
    }

    /// 这一项现在改不了：交回提示的话。终端改不了的写「终端暂不支持修改此项」；只能写系统配置、管理员模式又没开的，管理员
    /// 写「要先进入管理员模式」，不是管理员的照「终端暂不支持修改此项」（按了也进不去）。
    pub(in crate::settings) fn blocked(&self, item: &Item, texts: &Texts) -> Option<String> {
        if !item.writable() || item.system_only() && self.admin.is_none() {
            return Some(texts.more.no_editor.clone());
        }
        (item.system_only() && !self.admin_on()).then(|| texts.admin[1].clone())
    }

    /// 这一项改的是系统那一层：管理员模式开着、清单里能写系统的。
    pub(in crate::settings) fn writes_system(&self, key: &str) -> bool {
        self.admin_on() && self.item(key).is_some_and(Item::allows_system)
    }

    /// 存以前定这一份写哪一层：开着的写系统，这一份里有只能写个人的项（界面语言这类）除外；关着的写个人。清单里没有的键
    /// （供应商、模型、池那几页的）两层都能写。
    pub(in crate::settings) fn pick_layer(&mut self) {
        let personal_only = |key: &String| self.item(key).is_some_and(|i| !i.allows_system());
        self.draft.system = self.admin_on() && !self.draft.keys().any(personal_only);
    }
}
