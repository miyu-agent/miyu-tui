//! 先画后连（蓝图 `tui.md`「连核心」第 1 条，2026-10-15 冷启动查出：首页要等连上核心、问到配置才画，冷启动时核心起来就要
//! 二三十毫秒）：上次读到的、画首页要用的几样记在终端的缓存里（`<缓存目录>/tui/look/<数据根的路径算的一串>.json`，一个数据根
//! 一份，同时开着几个数据根的不互相盖；文件里也记着是哪个数据根，对不上的不认），起来时
//! 连上以前照它画；连上以后照核心给的换，一样的看不出来。记的：引导走过没有、终端自己的配置项（配色、明暗、主色、图标、
//! 吉祥物和时间线的开关、开局级别）、界面语言、默认人格和人格列表（首页的头像、跟随头像的主色）、预设列表、壁纸的候选主色。
//! 人格、预设两份列表也让新会话开头的框连上就开，不等核心（2026-10-22 项目主人：进界面到人格框出来慢，核心冷启动时列人格
//! 要近一秒）。
//! 读到的和记着的不一样时当场写（关窗口时界面是被直接结束的，等不到退出再写）；连上以后什么都没读到的不写。

use std::path::PathBuf;

use miyu_client::places::{DataRoot, Env};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::App;
use crate::core::Persona;

/// 记着的样子。
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Look {
    /// 哪个数据根的：换了数据根的不认。
    root: String,
    /// 引导走过了。
    welcomed: bool,
    /// 终端自己的配置项：`config.get` 回应里的 `items`（读到过的都在）。
    pub(super) items: Map<String, Value>,
    /// 界面语言：`auto` 或者语言代码；还没读到的是 `None`。
    language: Option<String>,
    /// 默认人格的编号。
    persona: Option<String>,
    /// 人格列表；还没读到的是 `None`。
    personas: Option<Vec<Persona>>,
    /// 预设列表；还没读到的是 `None`（以前记的没有这一格）。
    #[serde(default)]
    presets: Option<Vec<Persona>>,
    /// 壁纸的候选主色（跟随壁纸的，找到以前先照它）。
    wallpaper: Vec<u32>,
    /// 最近那个会话用的人格、预设：会话列表到以前首页照它画、框照它停（2026-10-19：无人格也先闪一帧默认人格的头像）。
    #[serde(default)]
    last: Option<Last>,
    /// 配置页的清单（`config.schema` 的回应）、预设列表：配置页起来时照它先画整张主菜单（2026-10-21 项目主人：「/config
    /// 打开配置菜单闪了一下」）。只记清单和名字，不记配置的值（值里有令牌、请求头）。
    #[serde(default)]
    pub(super) settings: (Option<Value>, Option<Value>),
    /// 现在用的主题的整份（选了主题包的；默认主题不记）：起来第一帧照它画（2026-10-21 项目主人：选了主题包的起来先闪一下
    /// 默认主题）。连上以后照常读包，改了、卸了再换。
    #[serde(default)]
    theme: Option<Value>,
    /// 上一次写下的：一样的不再写。
    #[serde(skip)]
    written: Vec<u8>,
}

/// 最近那个会话用的人格、预设；没带人格的 `persona` 是 `None`。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Last {
    persona: Option<String>,
    preset: Option<String>,
}

/// 记在哪：机器的缓存目录下的 `tui/look/<这个数据根的路径算的一串>.json`。
fn file() -> Option<PathBuf> {
    let root = root()?;
    miyu_client::places::cache_root(&Env::current())
        .ok()
        .map(|dir| {
            dir.join("tui")
                .join("look")
                .join(format!("{:016x}.json", fnv(&root)))
        })
}

/// 这一回的数据根。
fn root() -> Option<String> {
    DataRoot::locate(&Env::current())
        .ok()
        .map(|root| root.path().display().to_string())
}

/// FNV-1a：照路径算文件名，跨版本、跨进程都一样（标准库的哈希不保证）。
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

impl App {
    /// 起来时、连上以前：照上次记下的样子摆好（同一个数据根的才认）。
    pub fn restore_look(&mut self) {
        let Some(look) = file()
            .and_then(|f| std::fs::read(f).ok())
            .and_then(|bytes| serde_json::from_slice::<Look>(&bytes).ok())
            .filter(|look| Some(&look.root) == root().as_ref())
        else {
            return;
        };
        // 人格、壁纸先摆好：主题照它们挑主色（跟随头像、跟随壁纸的），不然第一帧照 tokyonight、下一帧才换。
        self.talk.persona.default.clone_from(&look.persona);
        // 两份列表照核心交回的那样摆（人格前面加「无人格」），连上就开框（`new_session.rs` 的 `picks_configured`）。
        if let Some(list) = look.personas.clone() {
            self.listed(super::new_session::Kind::Persona, list);
        }
        if let Some(list) = look.presets.clone() {
            self.listed(super::new_session::Kind::Preset, list);
        }
        if let Some(last) = &look.last {
            self.talk.persona.last = Some(last.persona.clone());
            self.talk.preset.last = Some(last.preset.clone());
        }
        self.screen.wallpaper.restore(&look.wallpaper);
        if let Some(language) = &look.language {
            self.language_from_config(language);
        }
        // 主题先换上：`layout`、`timeline` 照它重读，下面终端自己的配置项再盖上去。
        if let Some(theme) = &look.theme
            && crate::sheet::set(theme).is_ok()
            && self.config.retheme(theme).is_err()
        {
            drop(crate::sheet::set(&crate::sheet::default_value()));
        }
        let items = Value::Object(look.items.clone());
        self.apply_head_settings(&json!({ "items": items }));
        // 走过引导的：首页不等问 `ui.welcomed`，当场画（问到了没走过的照常进引导）。
        if look.welcomed {
            self.boxes.welcome_known = true;
            self.boxes.personas_known = look.personas.is_some();
        }
        self.link.look = look;
        self.link.look.written = std::fs::read(file().unwrap_or_default()).unwrap_or_default();
        // 记着走过引导的：人格框照记下的列表这就开，不等连上（`new_session.rs` 的 `ask_next`）。
        self.ask_next();
    }

    /// 读到的终端配置项记下（`config_asks.rs` 合起来那一条的回应）。
    pub(super) fn note_items(&mut self, got: &Value) {
        if let Some(items) = got["items"].as_object() {
            for (key, item) in items {
                self.link.look.items.insert(key.clone(), item.clone());
            }
        }
        self.save_look();
    }

    /// 读到引导走过没有。
    pub(super) fn note_welcomed(&mut self, welcomed: bool) {
        self.link.look.welcomed = welcomed;
        self.save_look();
    }

    /// 读到界面语言、默认人格。
    pub(super) fn note_head(&mut self, language: &str, persona: Option<&str>) {
        self.link.look.language = Some(language.to_string());
        self.link.look.persona = persona.map(str::to_string);
        self.save_look();
    }

    /// 读到人格列表。
    pub(super) fn note_personas(&mut self, list: &[Persona]) {
        self.boxes.personas_known = true;
        self.link.look.personas = Some(list.to_vec());
        self.save_look();
    }

    /// 读到预设列表。
    pub(super) fn note_presets(&mut self, list: &[Persona]) {
        self.link.look.presets = Some(list.to_vec());
        self.save_look();
    }

    /// 会话列表到了：记下最近那个会话用的（变了才写）。
    pub(super) fn note_last(&mut self) {
        let Some(latest) = self
            .talk
            .sessions_seen
            .as_deref()
            .and_then(crate::core::latest_session)
        else {
            return;
        };
        let last = Last {
            persona: latest.persona.clone(),
            preset: latest.preset.clone(),
        };
        self.talk.persona.last = Some(last.persona.clone());
        self.talk.preset.last = Some(last.preset.clone());
        if self.link.look.last.as_ref() != Some(&last) {
            self.link.look.last = Some(last);
            self.save_look();
        }
    }

    /// 换了主题：记下整份（默认主题是 `None`）。
    pub(super) fn note_theme(&mut self, theme: Option<&Value>) {
        if self.link.look.theme.as_ref() != theme {
            self.link.look.theme = theme.cloned();
            self.save_look();
        }
    }

    /// 配置页读回了清单、预设列表：变了的记下。
    pub(super) fn note_settings(&mut self, raw: (Option<&Value>, Option<&Value>)) {
        let look = &mut self.link.look.settings;
        let schema = raw.0.filter(|v| look.0.as_ref() != Some(*v));
        let presets = raw.1.filter(|v| look.1.as_ref() != Some(*v));
        if schema.is_none() && presets.is_none() {
            return;
        }
        if let Some(schema) = schema {
            look.0 = Some(schema.clone());
        }
        if let Some(presets) = presets {
            look.1 = Some(presets.clone());
        }
        self.save_look();
    }

    /// 壁纸找好了：记下候选主色。
    pub(super) fn note_wallpaper(&mut self) {
        self.link.look.wallpaper = self.screen.wallpaper.seeds().to_vec();
        self.save_look();
    }

    /// 写下这一回的样子：和上一次写的一样的不写；连上以后什么都没读到的不写（留着上一回的）。写不成的不管，下次照旧等核心。
    pub(super) fn save_look(&mut self) {
        if self.link.look.items.is_empty() {
            return;
        }
        let (Some(file), Some(root)) = (file(), root()) else {
            return;
        };
        let look = &mut self.link.look;
        look.root = root;
        let Ok(body) = serde_json::to_vec(&*look) else {
            return;
        };
        if body == look.written {
            return;
        }
        look.written.clone_from(&body);
        let temp = file.with_extension("json.tmp");
        let written = file
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&temp, body))
            .and_then(|()| std::fs::rename(&temp, &file));
        if written.is_err() {
            // 写不成：下次起来照旧等核心。
        }
    }
}
