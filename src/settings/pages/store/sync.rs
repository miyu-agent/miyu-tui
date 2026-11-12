//! 刷新软件源（蓝图「配置页」第 43c 条，2026-11-12 项目主人：「这个界面没法进行 miyu pkg update 之类更新软件列表数据之类的操作，
//! 加个 r 快捷键」）：「下载软件」「已安装」两页按 `r` 发 `package.sync {force: true}`（同 `miyu pkg update`），回来以后重读软件源、
//! 装着的、能升级的，光标停回原来那个包。

use serde_json::{Value, json};

use super::Ask;
use super::flow::say;
use crate::core::Refusal;
use crate::settings::{Settings, Texts, Tone, Waiting};

impl Settings {
    /// 按了 `r`：正在刷的不再发。
    pub(super) fn store_sync(&mut self, texts: &Texts) {
        if self.more.store.syncing {
            return;
        }
        self.more.store.syncing = true;
        self.say(say(&texts.more.store.status, "syncing"), Tone::Busy);
        let ask = Waiting::Store(Ask::Sync);
        self.ask("package.sync", json!({"force": true}), ask);
    }

    /// 刷完了：说一句（连不上、用的是本机那份的照实说），重读；拒了写原话。
    pub(super) fn store_synced(&mut self, result: Result<Value, Refusal>, texts: &Texts) {
        self.more.store.syncing = false;
        let words = &texts.more.store.status;
        match result {
            Ok(got) if got.get("stale").is_some_and(|s| !s.is_null()) => {
                self.say(say(words, "sync_stale"), Tone::Note);
            }
            Ok(got) => {
                let n = got.get("packages").and_then(Value::as_u64).unwrap_or(0);
                let line = say(words, "synced").replace("{n}", &n.to_string());
                self.say(line, Tone::Good);
            }
            Err(refusal) => return self.say(refusal.message, Tone::Bad),
        }
        self.store_reload();
    }
}
