//! 「软件包」三页的回应（蓝图「配置页」第 43c 条）：读来的列表、详情记下；看一眼的攒齐了问人；做的成了写状态行、重读，
//! 拒了写拒绝的原话。

use serde_json::Value;

use super::flow::{Doing, Flow, say};
use super::{Listing, read};
use crate::core::Refusal;
use crate::settings::{Settings, Texts, Tone};

/// 等着哪个回应。
#[derive(Debug, Clone)]
pub enum Ask {
    /// `package.search`。
    Search,
    /// `package.upgrades`。
    Upgrades,
    /// `package.info`：是不是软件源里的、编号。
    Info(bool, String),
    /// 看一眼（`preview: true`）。
    Preview,
    /// 装这一个（几个的一个个来）。
    Install(String),
    /// 卸、升级、批了能力再开：一次就完。
    Done,
    /// 空格开关：编号、开还是关。
    Toggle(String, bool),
}

impl Settings {
    /// 「软件后台」跑完了：开了浏览器的说一句；开不了的把网址写出来；没跑成的写为什么。
    pub fn store_web(
        &mut self,
        reply: &crate::backstage::Reply,
        opened: Result<bool, &String>,
        texts: &Texts,
    ) {
        let words = &texts.more.store;
        match (opened, &reply.url) {
            (Ok(true), _) => {
                let line = say(&words.status, "opened").replace("{name}", &reply.name);
                self.say(line, Tone::Good);
            }
            (Ok(false), Ok(url)) => {
                let line = say(&words.status, "no_browser").replace("{url}", url);
                self.say(line, Tone::Note);
            }
            (_, Err(why)) | (Err(why), _) => self.say(why.clone(), Tone::Bad),
        }
    }

    /// 回应到了。
    pub(in crate::settings) fn store_answer(
        &mut self,
        ask: Ask,
        result: Result<Value, Refusal>,
        texts: &Texts,
    ) {
        let store = &mut self.more.store;
        match (ask, result) {
            (Ask::Search, Ok(got)) => store.source_read(read::remotes(&got)),
            (Ask::Search, Err(refusal)) => store.source = Listing::Failed(refusal.message),
            (Ask::Upgrades, got) => {
                store.upgrades = got.map(|g| read::upgrades(&g)).unwrap_or_default();
            }
            (Ask::Info(source, id), Ok(got)) => {
                store.infos.insert((source, id), read::info(&got));
            }
            (Ask::Info(..), Err(_)) => {}
            (Ask::Preview, result) => self.store_previewed(result),
            (Ask::Install(id), Ok(_)) => self.store_installed(id, texts),
            (Ask::Done, Ok(got)) => self.store_done(&got, texts),
            (Ask::Toggle(id, on), result) => self.store_toggled(&id, on, result, texts),
            (Ask::Install(_) | Ask::Done, Err(refusal)) => {
                self.more.store.flow = None;
                self.say(refusal.message, Tone::Bad);
                self.store_reload();
            }
        }
    }

    /// 看一眼回来一个：攒齐了问人；拒了的整件不做，写拒绝的原话。
    fn store_previewed(&mut self, result: Result<Value, Refusal>) {
        let Some(Flow::Previewing {
            doing,
            left,
            mut previews,
        }) = self.more.store.flow.take()
        else {
            return;
        };
        let got = match result {
            Ok(got) => got,
            Err(refusal) => return self.say(refusal.message, Tone::Bad),
        };
        previews.push(got);
        self.more.store.flow = Some(if left > 1 {
            Flow::Previewing {
                doing,
                left: left - 1,
                previews,
            }
        } else {
            Flow::Confirm { doing, previews }
        });
    }

    /// 装好了一个：还有的接着装，都好了写状态行、清勾选、重读。
    fn store_installed(&mut self, id: String, texts: &Texts) {
        let name = self.more.store.remote(&id).map_or(id, |p| p.name.clone());
        let Some(Flow::Running {
            doing,
            mut queue,
            mut done,
            progress,
        }) = self.more.store.flow.take()
        else {
            return;
        };
        done.push(name);
        if !queue.is_empty() {
            let next = queue.remove(0);
            self.store_install_one(next);
            self.more.store.flow = Some(Flow::Running {
                doing,
                queue,
                done,
                progress,
            });
            return;
        }
        let words = &texts.more.store;
        let sep = say(&words.values, "sep");
        let line = say(&words.status, "installed").replace("{names}", &done.join(&sep));
        self.more.store.picked.clear();
        self.say(line, Tone::Good);
        self.store_reload();
    }

    /// 卸、升级、批了能力再开，成了：升级的照回应的 `upgraded` 写升了哪几个。
    fn store_done(&mut self, got: &Value, texts: &Texts) {
        let Some(Flow::Running { doing, .. }) = self.more.store.flow.take() else {
            return;
        };
        let words = &texts.more.store;
        let name = |id: &str| {
            self.more
                .store
                .local(id)
                .map_or(id, |p| p.name.as_str())
                .to_string()
        };
        let line = match &doing {
            Doing::Remove(id) => say(&words.status, "removed").replace("{name}", &name(id)),
            Doing::Enable(id, _) => say(&words.status, "on").replace("{name}", &name(id)),
            Doing::Upgrade(_) => {
                let sep = say(&words.values, "sep");
                let names: Vec<String> = got["upgraded"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|u| u["package"].as_str())
                    .map(name)
                    .collect();
                say(&words.status, "upgraded").replace("{names}", &names.join(&sep))
            }
            Doing::Install(_) => String::new(),
        };
        self.say(line, Tone::Good);
        self.store_reload();
    }

    /// 空格开关回来了：成了写状态行、重读；要批能力的问人；别的拒了写原话。
    fn store_toggled(&mut self, id: &str, on: bool, result: Result<Value, Refusal>, texts: &Texts) {
        let words = &texts.more.store;
        let name = self
            .more
            .store
            .local(id)
            .map_or(id, |p| p.name.as_str())
            .to_string();
        match result {
            Ok(_) => {
                let key = if on { "on" } else { "off" };
                self.say(say(&words.status, key).replace("{name}", &name), Tone::Good);
                self.store_reload();
            }
            Err(refusal) if refusal.reason.as_deref() == Some("needs_approval") => {
                let caps = refusal.data["capabilities"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|c| c.as_str().map(str::to_string))
                    .collect();
                self.more.store.flow = Some(Flow::Confirm {
                    doing: Doing::Enable(id.to_string(), caps),
                    previews: Vec::new(),
                });
            }
            Err(refusal) => self.say(refusal.message, Tone::Bad),
        }
    }
}
