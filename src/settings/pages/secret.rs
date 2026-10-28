//! 密钥项（蓝图「配置页」第 48 条，`type` 是 `secret`；2026-10-17 项目主人照推荐定「做，输入时遮住」，起因是接入通讯平台 的令牌）：
//! 那一行只写「已设置」「未设置」；`Enter` 开一行字的编辑窗，打的字写成 `•`；`Enter` 存：先 `secret.set` 存成新名字的密钥，
//! 再把这一项写成指向它的引用，和供应商页存 key 同一份草稿（`draft.rs`）。明文只在编辑窗和发 `secret.set` 那一刻。

use serde_json::Value;

use super::choose::LineEdit;
use super::schema::Item;
use super::target::Target;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts};

/// 这一项是密钥。
pub fn is_secret(item: &Item) -> bool {
    item.kind == "secret"
}

/// 编辑窗里显示的字：密钥的一个字一个 `•`（照主题的 `marks.secret`）。
pub fn masked(text: &str) -> String {
    crate::sheet::mark(|m| &m.secret).repeat(text.chars().count())
}

impl Settings {
    /// 开窗：空着的，不露原来的引用。
    pub(in crate::settings) fn open_secret(&mut self, item: &Item) {
        let line = LineEdit::open(&item.key, &item.name, false, &Value::Null);
        self.popup = Some(Popup::Line(line.to(Target::Secret)));
    }

    /// 编辑窗写好了：空的不存，不空的记进草稿当场存。不是密钥的交回 `false`。
    pub(in crate::settings) fn secret_write(
        &mut self,
        target: &Target,
        key: &str,
        value: &Value,
        texts: &Texts,
    ) -> bool {
        if *target != Target::Secret {
            return false;
        }
        let text = value.as_str().unwrap_or_default();
        if text.is_empty() {
            return true;
        }
        // 只能写系统配置的项（im 的令牌这类）引用写系统那一层（`saving.rs` 的 `system_only`）。
        self.draft.system = self.system_only(key);
        self.draft.set_secret_at(key, text.to_string());
        self.save(false, texts);
        true
    }
}

#[cfg(test)]
mod tests;
