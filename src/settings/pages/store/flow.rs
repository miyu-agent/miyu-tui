//! 先问再做（蓝图「配置页」第 43c 条，2026-10-31 项目主人照推荐定：装之前问一句）：装、卸、升级先带 `preview: true` 看一眼，
//! 预览块写出要做的、问「继续？」；`y` 才真做，装、升级照 `package.progress` 写进度。开扩展回 `needs_approval` 的也在这里问。
//! 做完重读列表（`answer.rs`）。

use serde_json::{Value, json};

use super::answer::Ask;
use super::{Item, Listing, Tab};
use crate::core::Progress;
use crate::settings::{Settings, Texts, Tone, Waiting};

/// 要做哪一件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Doing {
    /// 从软件源装这几个（编号，照列表的先后）。
    Install(Vec<String>),
    /// 卸这一个。
    Remove(String),
    /// 升级这一个；`None` 是全部。
    Upgrade(Option<String>),
    /// 开这一个、批这几项能力（`data.capabilities` 的原样）。
    Enable(String, Vec<String>),
}

/// 正在问、正在做。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Flow {
    /// 去看一眼了：还差几个没回来，回来了的预览（照问的先后）。
    Previewing {
        /// 要做的。
        doing: Doing,
        /// 还差几个。
        left: usize,
        /// 回来了的。
        previews: Vec<Value>,
    },
    /// 都回来了，等人按 `y`、`n`。
    Confirm {
        /// 要做的。
        doing: Doing,
        /// 看一眼的回应。
        previews: Vec<Value>,
    },
    /// 在做。
    Running {
        /// 在做的。
        doing: Doing,
        /// 还要装的（装几个的时候一个个来）。
        queue: Vec<String>,
        /// 做好了的名字。
        done: Vec<String>,
        /// 每个包到了哪一段（`package.progress`）。
        progress: Vec<Progress>,
    },
}

impl Settings {
    /// 「下载软件」`Enter`：装勾了的，没勾的装光标那一个；拉黑了的不装，都装着、版本也一样的不问。
    pub(super) fn store_install(&mut self, texts: &Texts) {
        let store = &self.more.store;
        let words = &texts.more.store;
        let rows = store.rows();
        let chosen: Vec<&super::Remote> = if store.picked.is_empty() {
            store.current().into_iter().collect::<Vec<_>>()
        } else {
            rows.iter()
                .filter(|r| store.picked.contains(r.id()))
                .copied()
                .collect()
        }
        .into_iter()
        .filter_map(|r| match r {
            Item::Remote(p) => Some(p),
            Item::Local(_) => None,
        })
        .collect();
        if let Some(p) = chosen.iter().find(|p| p.blocked.is_some()) {
            let why = p.blocked.clone().unwrap_or_default();
            let line = say(&words.status, "blocked").replace("{why}", &why);
            return self.say(line, Tone::Bad);
        }
        let fresh: Vec<String> = chosen
            .iter()
            .filter(|p| p.installed.as_deref() != Some(p.version.as_str()))
            .map(|p| p.id.clone())
            .collect();
        if fresh.is_empty() {
            if let Some(p) = chosen.first() {
                let line = say(&words.status, "latest").replace("{name}", &p.name);
                self.say(line, Tone::Note);
            }
            return;
        }
        for id in &fresh {
            let params = json!({"source": id, "preview": true});
            self.ask("package.install", params, Waiting::Store(Ask::Preview));
        }
        self.more.store.flow = Some(Flow::Previewing {
            left: fresh.len(),
            doing: Doing::Install(fresh),
            previews: Vec::new(),
        });
    }

    /// 「已安装」`d`：先看一眼卸了会删掉什么。
    pub(super) fn store_remove(&mut self) {
        let Some(id) = self.more.store.current().map(|r| r.id().to_string()) else {
            return;
        };
        let params = json!({"package": id, "preview": true});
        self.ask("package.remove", params, Waiting::Store(Ask::Preview));
        self.more.store.flow = Some(Flow::Previewing {
            doing: Doing::Remove(id),
            left: 1,
            previews: Vec::new(),
        });
    }

    /// 「已安装」`u`（`all` 是 `U`）：没有能升的不问。
    pub(super) fn store_upgrade(&mut self, all: bool, texts: &Texts) {
        let store = &self.more.store;
        let words = &texts.more.store;
        let one = store
            .current()
            .map(|r| (r.id().to_string(), r.name().to_string()));
        let target = if all {
            None
        } else {
            match one {
                Some((id, _)) if store.upgrades.contains_key(&id) => Some(id),
                Some((_, name)) => {
                    let line = say(&words.status, "latest").replace("{name}", &name);
                    return self.say(line, Tone::Note);
                }
                None => return,
            }
        };
        if all && store.upgrades.is_empty() {
            return self.say(say(&words.status, "all_latest"), Tone::Note);
        }
        let params = upgrade_params(target.as_deref(), true);
        self.ask("package.upgrade", params, Waiting::Store(Ask::Preview));
        self.more.store.flow = Some(Flow::Previewing {
            doing: Doing::Upgrade(target),
            left: 1,
            previews: Vec::new(),
        });
    }

    /// 「已安装」空格：开着的关、关着的开（不问；回 `needs_approval` 的再问，`answer.rs`）。
    pub(super) fn store_toggle(&mut self) {
        let Some(p) = self
            .more
            .store
            .current()
            .and_then(|r| self.more.store.local(r.id()))
        else {
            return;
        };
        let (method, on) = if p.on {
            ("package.disable", false)
        } else {
            ("package.enable", true)
        };
        let id = p.id.clone();
        self.ask(
            method,
            json!({"package": id}),
            Waiting::Store(Ask::Toggle(id, on)),
        );
    }

    /// 问的时候按 `y`、`Enter`：真做。
    pub(super) fn store_go(&mut self) {
        let Some(Flow::Confirm { doing, .. }) = self.more.store.flow.take() else {
            return;
        };
        let mut queue = Vec::new();
        match &doing {
            Doing::Install(ids) => {
                queue = ids.clone();
                let first = queue.remove(0);
                self.store_install_one(first);
            }
            Doing::Remove(id) => {
                let params = json!({"package": id});
                self.ask("package.remove", params, Waiting::Store(Ask::Done));
            }
            Doing::Upgrade(target) => {
                let params = upgrade_params(target.as_deref(), false);
                self.ask("package.upgrade", params, Waiting::Store(Ask::Done));
            }
            Doing::Enable(id, caps) => {
                let params = json!({"package": id, "approve": caps});
                self.ask("package.enable", params, Waiting::Store(Ask::Done));
            }
        }
        self.more.store.flow = Some(Flow::Running {
            doing,
            queue,
            done: Vec::new(),
            progress: Vec::new(),
        });
    }

    /// 装一个（几个的一个个来）。
    pub(super) fn store_install_one(&mut self, id: String) {
        let params = json!({"source": id});
        self.ask("package.install", params, Waiting::Store(Ask::Install(id)));
    }

    /// 问的时候按 `n`、`Esc`：不做。
    pub(super) fn store_cancel(&mut self, texts: &Texts) {
        self.more.store.flow = None;
        self.say(say(&texts.more.store.status, "cancelled"), Tone::Note);
    }

    /// 装包的进度推来了：记在正在做的那一件上。
    pub fn store_progress(&mut self, progress: &Progress) {
        if let Some(Flow::Running { progress: all, .. }) = &mut self.more.store.flow {
            match all.iter_mut().find(|p| p.package == progress.package) {
                Some(p) => *p = progress.clone(),
                None => all.push(progress.clone()),
            }
        }
    }

    /// 做完了（成了、拒了都算）：重读装着的、能升级的，读过软件源的再读一次；光标停回做之前的那个包。
    pub(super) fn store_reload(&mut self) {
        let store = &mut self.more.store;
        store.keep = store.current().map(|r| (store.tab, r.id().to_string()));
        store.infos.clear();
        store.asked.clear();
        let searched = store.source != Listing::Unasked;
        self.ask("package.list", json!({}), Waiting::Mascots);
        // 装卸以后设置项跟着多、少，有的还换了 `hidden`（核心 F-5 补、UH-1）：重读清单，包自己那一页、「有没有设置」照新的。
        self.ask("config.schema", json!({}), Waiting::Schema);
        self.ask("package.upgrades", json!({}), Waiting::Store(Ask::Upgrades));
        if searched {
            self.ask("package.search", json!({}), Waiting::Store(Ask::Search));
        }
    }

    /// 进「软件包」、换到一页：要还没要过的（软件源只在进「下载软件」时要）。
    pub(in crate::settings) fn store_load(&mut self) {
        if self.more.store.tab == Tab::Source && self.more.store.source == Listing::Unasked {
            self.more.store.source = Listing::Loading;
            self.ask("package.search", json!({}), Waiting::Store(Ask::Search));
        }
        if let Some((source, id)) = self.more.store.wants_info() {
            let params = json!({"package": id, "source": source});
            self.ask(
                "package.info",
                params,
                Waiting::Store(Ask::Info(source, id)),
            );
        }
    }
}

/// `package.upgrade` 的参数：写了编号的只升它。
fn upgrade_params(target: Option<&str>, preview: bool) -> Value {
    let mut params = json!({"preview": preview});
    if let Some(id) = target {
        params["packages"] = json!([id]);
    }
    params
}

/// 一句字：字表少写了的是空的。
pub(super) fn say(table: &std::collections::HashMap<String, String>, key: &str) -> String {
    super::StoreTexts::say(table, key)
}
