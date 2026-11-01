//! 测一家供应商、重取它的模型列表（蓝图「配置页」第 11 条 `r`）：焦点在模型栏的试选中的模型，在供应商栏由核心挑；还没存的
//! 新供应商先要存。回来了照结果写状态行。

use serde_json::{Value, json};

use super::{Refusal, Settings, Texts, Tone, Waiting, nav};

impl Settings {
    /// 测一家（`r`）：焦点在模型栏的试选中的模型，在供应商栏由核心挑；还没存的新供应商先要存。
    pub fn test(&mut self, texts: &Texts) {
        let Some(p) = self.nav.provider(&self.view) else {
            return;
        };
        let name = p.shown().to_string();
        if self.draft.new_providers.contains(&p.id) {
            self.say(texts.status("save_first"), Tone::Bad);
            return;
        }
        if !self.online {
            self.say(texts.status("offline"), Tone::Bad);
            return;
        }
        let mut params = json!({"provider": p.id});
        if self.nav.focus(&self.view) == nav::Col::Model
            && let Some(m) = self.nav.model(&self.view)
        {
            params["model"] = json!(m.name);
        }
        self.say(texts.status("testing").replace("{name}", &name), Tone::Busy);
        self.ask("provider.test", params, Waiting::Test(name));
    }

    pub(super) fn tested(&mut self, name: &str, result: Result<Value, Refusal>, texts: &Texts) {
        match result {
            Ok(got) if got["ok"] == true => {
                let n = got["models"].as_array().map_or(0, Vec::len);
                // 列出了模型的说列出几个；列不出、拿选着的模型发了一句通了的说那个模型（核心 8-11 五补）。
                let text = if n > 0 {
                    texts.status("tested").replace("{n}", &n.to_string())
                } else {
                    let model = got["model"].as_str().unwrap_or_default();
                    texts.status("tested_request").replace("{model}", model)
                };
                self.say(text.replace("{name}", name), Tone::Good);
                self.reload();
            }
            // 列不出模型、给一个模型名就能再试的：请人在模型栏选一个再按 `r`。
            Ok(got) if got["need_model"] == true => {
                let text = texts.status("need_model").replace("{name}", name);
                self.say(text, Tone::Note);
            }
            Ok(got) => {
                let why = got["error"]["message"].as_str().unwrap_or_default();
                let text = texts
                    .status("test_failed")
                    .replace("{name}", name)
                    .replace("{why}", why);
                self.say(text, Tone::Bad);
            }
            Err(refusal) => {
                let text = texts
                    .status("test_failed")
                    .replace("{name}", name)
                    .replace("{why}", &refusal.message);
                self.say(text, Tone::Bad);
            }
        }
    }

    pub(super) fn refreshed(
        &mut self,
        id: &str,
        name: &str,
        result: Result<Value, Refusal>,
        texts: &Texts,
    ) {
        let got = result.ok();
        let n = got
            .as_ref()
            .and_then(|g| g["providers"].as_array())
            .and_then(|all| all.iter().find(|p| p["id"] == id))
            .and_then(|p| p["models"].as_array())
            .map_or(0, Vec::len);
        let (key, tone) = if n > 0 {
            ("tested", Tone::Good)
        } else {
            ("fetch_failed", Tone::Bad)
        };
        let text = texts
            .status(key)
            .replace("{name}", name)
            .replace("{n}", &n.to_string());
        self.say(text, tone);
        self.reload();
    }
}
