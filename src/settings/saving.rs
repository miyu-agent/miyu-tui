//! 配置页怎么存（蓝图「配置页」第 16、24 条）：先把新贴的 key 存进密钥库，全成了再一次 `config.set`；存成了重读，
//! 没存成的这一次的改动扔掉、原因写出来。在途时又改了又要存的，回话以后接着存（2026-11-28 审计第 3 条）。从 `mod.rs` 分出来。

use serde_json::{Value, json};

use super::data::Data;
use super::draft::Draft;
use super::{Popup, Settings, Texts, Tone, Waiting, forms, merge};
use crate::core::Refusal;

impl Settings {
    /// 存（悬浮窗里 `s`、`q`，删除、选默认模型确认了也是）：先把新贴的 key 存进密钥库，全成了再一次 `config.set`
    /// （「配置页」第 24 条）。`close` 是存成了关掉悬浮窗。
    pub fn save(&mut self, close: bool, texts: &Texts) {
        // 上一次还没回话：不重发，记着回话以后接着存这之间又改的（[`Settings::resave`]；原来直接不管了，回话一到连这一项
        // 一起清掉、写「已保存」）。草稿这时是空的也记着：在途时把刚改的改回原值，草稿里那一项没了，可核心那头存成了的是
        // 刚改的那个值，回话以后照 [`Draft::since`] 还要改回来。
        if self
            .waiting
            .values()
            .any(|w| matches!(w, Waiting::Save | Waiting::Secret(..) | Waiting::Rename))
        {
            self.queued = Some(self.queued == Some(true) || close);
            return;
        }
        if self.draft.is_empty() {
            if close {
                self.popup = None;
            }
            return;
        }
        let Some(data) = self.data.as_ref().filter(|_| self.online) else {
            self.draft.clear();
            self.refresh();
            self.failed(texts.status("offline"));
            return;
        };
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
            self.rename_provider(self.draft.rename.clone());
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

    /// 没存成：这一次的改动扔掉（界面照回读来的），原因写在开着的编辑窗里，没开着的写在状态行。这一次发出去以后又改的
    /// 留着（[`Settings::settle`]）；还没发出去就没成的（存 key 没成、连不上）整份扔掉。
    pub(super) fn failed(&mut self, text: String) {
        self.close_after_save = false;
        if self.sent.is_some() {
            self.settle(false);
        } else {
            self.draft.clear();
            self.queued = None;
        }
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
        self.sent = Some((self.draft.sending(&self.named), params.clone()));
        self.ask("config.set", params, Waiting::Save);
    }

    /// 在途的这一次回话了：草稿里只留下发出去以后又改的（[`Draft::since`]），交回发出去的那一份。`saved` 是存成了：界面上看的
    /// 那一份先当成读来的——核心那头现在存着的，读来的叠上发出去的——等重读回来再换成核心的（不闪一下旧的），又改的照它算。
    fn settle(&mut self, saved: bool) -> Draft {
        let Some((sent, request)) = self.sent.take() else {
            return std::mem::take(&mut self.draft);
        };
        let Some(old) = self.data.take() else {
            self.draft.clear();
            return sent;
        };
        let base = if saved {
            applied(&old, &request)
        } else {
            old.clone()
        };
        self.draft = self.draft.since(&sent, &old, &base);
        self.data = Some(if saved {
            let mut shown = merge::merge(&old, &sent);
            shown.system = base.system;
            shown
                .secrets
                .extend(self.named.iter().map(|(_, name)| name.clone()));
            shown
        } else {
            old
        });
        sent
    }

    /// 回话以后：在途时又要存过、这之间又改了的接着存；不然 `close`（这一次存成了关窗）的关掉悬浮窗。
    fn resave(&mut self, close: bool, texts: &Texts) {
        let queued = self.queued.take();
        let close = close || queued == Some(true);
        if queued.is_some() && !self.draft.is_empty() {
            self.save(close, texts);
        } else if close {
            self.popup = None;
        }
    }

    /// 没存成以后：在途时又要存过、这之间又改了的照样接着存（各是各的一次），没存成的那一次的原因留在窗里、状态行上。
    fn resave_after_failure(&mut self, texts: &Texts) {
        if let Some(close) = self.queued.take()
            && !self.draft.is_empty()
        {
            self.save(close, texts);
        }
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
        // 存的里头有要重启核心才生效的（`config.set` 回应 `keys` 的 `applies` 是 `restart`，核心 CPU-1 的 `core.workers`）：回执说清楚。
        let restart = result.as_ref().is_ok_and(|got| {
            got["keys"]
                .as_object()
                .is_some_and(|keys| keys.values().any(|k| k["applies"] == "restart"))
        });
        match result {
            // 还要改供应商编号：别的改动照旧编号存成了，接着改编号，改完再算存完。
            Ok(_)
                if self
                    .sent
                    .as_ref()
                    .is_some_and(|(sent, _)| sent.rename.is_some()) =>
            {
                let sent = self.settle(true);
                self.named.clear();
                // 改编号这一件还在路上：草稿里照旧记着（编辑窗照它认这一家现在的编号，`forms/apply.rs`）。
                self.draft.rename.clone_from(&sent.rename);
                self.rename_provider(sent.rename);
            }
            Ok(got) => {
                let before = self.data.clone();
                let sent = self.settle(true);
                let reconnect = before.map(|d| sent.reconnects(&d)).unwrap_or_default();
                self.named.clear();
                let said = if restart { "saved_restart" } else { "saved" };
                match warnings(&got) {
                    None => self.say(texts.status(said), Tone::Good),
                    Some(warned) => self.say(
                        texts
                            .status("saved_warned")
                            .replace("{saved}", &texts.status(said))
                            .replace("{warnings}", &warned),
                        Tone::Note,
                    ),
                }
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
                let close = std::mem::take(&mut self.close_after_save);
                self.resave(close, texts);
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
                self.resave_after_failure(texts);
            }
        }
    }

    /// 改供应商编号（核心 8-34，2026-10-22 项目主人：「最好是能改」）：核心一次改完配置、密钥引用、默认模型和池里的引用，
    /// 两层各推一条 `config.changed`。
    fn rename_provider(&mut self, rename: Option<(String, String)>) {
        if let Some((from, to)) = rename.clone() {
            let sent = Draft::renaming(rename);
            self.sent = Some((sent, json!({"changes": []})));
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
                self.settle(true);
                self.say(texts.status("saved"), Tone::Good);
                self.reload();
                self.refresh();
                let close = std::mem::take(&mut self.close_after_save);
                self.resave(close, texts);
            }
            Err(refusal) => {
                self.failed(refusal.message);
                self.resave_after_failure(texts);
            }
        }
    }
}

/// 核心那头存成了 `request`（发出去的 `config.set` 参数）以后读来的样子：系统配置照它改的每一项改过。
fn applied(old: &Data, request: &Value) -> Data {
    let mut base = old.clone();
    for change in request["changes"].as_array().into_iter().flatten() {
        let Some(key) = change["key"].as_str() else {
            continue;
        };
        if change["unset"] == true {
            base.system.remove(key);
        } else {
            base.system.insert(key.to_string(), change["value"].clone());
        }
    }
    base
}

/// 存成了的回应里核心的警告（核心 CK-1：改的那几项指着现在没有的人格、预设、知识库，照收不挡）：已经照连接的语言说，几句
/// 用空格连；没有的是 `None`。
fn warnings(reply: &Value) -> Option<String> {
    let said: Vec<&str> = reply["problems"]
        .as_array()?
        .iter()
        .filter_map(|problem| problem["message"].as_str())
        .collect();
    (!said.is_empty()).then(|| said.join(" "))
}

#[cfg(test)]
mod tests;
