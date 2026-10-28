//! 配置页怎么存（蓝图「配置页」第 16、24 条）：先把新贴的 key 存进密钥库，全成了再一次 `config.set`；存成了重读，
//! 没存成的这一次的改动扔掉、原因写出来。从 `mod.rs` 分出来。

use serde_json::{Value, json};

use super::{Popup, Settings, Texts, Tone, Waiting, forms};
use crate::core::Refusal;

impl Settings {
    /// 存（悬浮窗里 `s`、`q`，删除、选默认模型确认了也是）：先把新贴的 key 存进密钥库，全成了再一次 `config.set`
    /// （「配置页」第 24 条）。`close` 是存成了关掉悬浮窗。
    pub fn save(&mut self, close: bool, texts: &Texts) {
        if self.draft.is_empty() {
            if close {
                self.popup = None;
            }
            return;
        }
        // 只能写系统配置的项（运行日志级别这类，「配置页」第 33 条）写系统那一层：单用户以后不用进管理员模式（核心 SU-1，
        // 两层合并在 SU-3）；恢复默认的照删的那一层，已经定好了。
        if !self.draft.system {
            let system = self.draft.keys().any(|key| self.system_only(key));
            self.draft.system = system;
        }
        let Some(data) = self.data.as_ref().filter(|_| self.online) else {
            self.draft.clear();
            self.refresh();
            self.failed(texts.status("offline"));
            return;
        };
        if self
            .waiting
            .values()
            .any(|w| matches!(w, Waiting::Save | Waiting::Secret(..) | Waiting::Rename))
        {
            return;
        }
        // 密钥的名字带这一刻的毫秒数（「配置页」第 48 条，同网页）。
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        let secrets = self.draft.secrets(data, now);
        self.close_after_save = close;
        self.named.clear();
        self.say(texts.status("saving"), Tone::Busy);
        // 只改了供应商编号：直接改。
        if secrets.is_empty() && self.draft.only_renames() {
            self.rename_provider();
            return;
        }
        if secrets.is_empty() {
            self.send_config();
        } else {
            self.secrets_left = secrets.len();
            for (key, name, value) in secrets {
                let params = json!({"name": name, "value": value});
                self.ask("secret.set", params, Waiting::Secret(key, name));
            }
        }
    }

    /// 没存成：这一次的改动扔掉（界面照回读来的），原因写在开着的编辑窗里，没开着的写在状态行。
    pub(super) fn failed(&mut self, text: String) {
        self.close_after_save = false;
        self.draft.clear();
        self.refresh();
        match &mut self.popup {
            Some(Popup::Form(form)) => {
                form.error = Some(text);
                self.status = None;
            }
            _ => self.say(text, Tone::Bad),
        }
    }

    fn send_config(&mut self) {
        let Some(data) = &self.data else {
            return;
        };
        let params = self.draft.request(data, &self.named);
        self.ask("config.set", params, Waiting::Save);
    }

    pub(super) fn secret_saved(
        &mut self,
        key: String,
        name: String,
        result: Result<Value, Refusal>,
    ) {
        match result {
            // 这一次已经作废了（前面有一个没存成）：晚到的成功不再数，不然数到 0 会把那之后新改的草稿发出去（review，
            // 2026-10-11）。存进去的 key 留在密钥库里，下次存照样用得上。
            Ok(_) if self.secrets_left == 0 => {}
            Ok(_) => {
                self.named.push((key, name));
                self.secrets_left -= 1;
                if self.secrets_left == 0 && !self.draft.is_empty() {
                    self.send_config();
                }
            }
            Err(refusal) => {
                // 一个没存成：别的存成了的留在密钥库里，配置里的引用照旧。
                self.secrets_left = 0;
                self.failed(refusal.message);
            }
        }
    }

    pub(super) fn saved(&mut self, result: Result<Value, Refusal>, texts: &Texts) {
        match result {
            // 还要改供应商编号：别的改动照旧编号存成了，接着改编号，改完再算存完。
            Ok(_) if self.draft.rename.is_some() => {
                let rename = self.draft.rename.take();
                self.draft.clear();
                self.draft.rename = rename;
                self.named.clear();
                self.rename_provider();
            }
            Ok(_) => {
                // 存成了：界面上看的那一份先当成读来的，等重读回来再换成核心的（不闪一下旧的）。
                let reconnect = self
                    .data
                    .as_ref()
                    .map(|d| self.draft.reconnects(d))
                    .unwrap_or_default();
                self.data = Some(self.view.clone());
                self.draft.clear();
                self.named.clear();
                self.say(texts.status("saved"), Tone::Good);
                self.reload();
                self.refresh();
                // 新加的、换了地址、key 的：马上取模型列表，不用再按 r（2026-10-07 项目主人）。
                for id in reconnect {
                    let name = self
                        .view
                        .provider(&id)
                        .map_or(id.clone(), |p| p.shown().to_string());
                    self.say(
                        texts.status("fetching").replace("{name}", &name),
                        Tone::Busy,
                    );
                    let params = json!({"provider": id, "refresh": true});
                    self.ask("model.list", params, Waiting::Refresh(id, name));
                }
                if std::mem::take(&mut self.close_after_save) {
                    self.popup = None;
                }
            }
            Err(refusal) => {
                // 核心那一句是总的（「配置有几处不对」）：接上第一处具体的。
                let first = refusal.data["problems"][0]["message"].as_str();
                let text = if refusal.reason.as_deref() == Some("config_conflict") {
                    texts.status("conflict").replace(
                        "{current}",
                        &forms::plain(&refusal.data["current"]["value"]),
                    )
                } else if let Some(first) = first {
                    format!("{} {first}", refusal.message)
                } else {
                    refusal.message
                };
                self.failed(text);
            }
        }
    }

    /// 改供应商编号（核心 8-34，2026-10-22 项目主人：「最好是能改」）：核心一次改完配置、密钥引用、默认模型和池里的引用，
    /// 两层各推一条 `config.changed`。
    fn rename_provider(&mut self) {
        if let Some((from, to)) = self.draft.rename.clone() {
            self.ask(
                "provider.rename",
                json!({"from": from, "to": to}),
                Waiting::Rename,
            );
        }
    }

    /// 改编号回来了：成了照存成办（重读、关窗）；没成写核心的原话（编号已存在这类），这一次别的改动已经存了。
    pub(super) fn renamed(&mut self, result: Result<Value, Refusal>, texts: &Texts) {
        match result {
            Ok(_) => {
                self.draft.clear();
                self.say(texts.status("saved"), Tone::Good);
                self.reload();
                self.refresh();
                if std::mem::take(&mut self.close_after_save) {
                    self.popup = None;
                }
            }
            Err(refusal) => self.failed(refusal.message),
        }
    }
}

impl Settings {
    /// 清单里这一项只能写系统配置。
    pub(in crate::settings) fn system_only(&self, key: &str) -> bool {
        self.more
            .schema
            .as_ref()
            .and_then(|s| s.items.iter().find(|i| i.key == key))
            .is_some_and(super::pages::schema::Item::system_only)
    }
}
