//! 终端自己的配置项（`tui.*`，终端软件包清单 `resources/packages/tui/package.toml` 的 `[settings]`）：连上时、配置变了时
//! `config.get` 读。图标那一套照 `tui.icons` 换（蓝图 `tui.md`「图标」第 4 条）；时间线的几个开关照 `tui.timeline_*`
//! 盖过 `timeline.json`（「时间线」第 18 条）；吉祥物的两个开关照 `tui.mascot_*` 盖过 `layout.json`（「后台命令、子代理和
//! 侧边栏」第 7 条）。几样分开问：旧核心不认时间线那几个键时，整份拒，图标照样读得到。

use serde_json::Value;

use super::App;
use crate::config::{Layout, Timeline};
use crate::core::Update;
use crate::oobe::asker::TAG_BASE;

/// 读图标那一套的请求编号：在引导自己的编号下面，和「引导走过没有」的（`TAG_BASE - 1`）分开。
const ICONS_TAG: u64 = TAG_BASE - 2;
/// 读时间线那几个开关的请求编号。
const TIMELINE_TAG: u64 = TAG_BASE - 3;
/// 读吉祥物那两个开关的请求编号（头像要的是 `TAG_BASE - 5`，`avatars.rs`）。
const MASCOT_TAG: u64 = TAG_BASE - 4;
/// 读主题那几项的请求编号（`TAG_BASE - 6` 到 `- 8` 是换吉祥物的，`mascot_pick.rs`）。
const THEME_TAG: u64 = TAG_BASE - 9;
/// 读新会话开局级别的请求编号。
const LEVEL_TAG: u64 = TAG_BASE - 10;
/// 读默认的聊天模型的请求编号（底栏只要它的资料，`effort.rs`）。
const CHAT_TAG: u64 = TAG_BASE - 11;
/// 读按键模式的请求编号：单问，旧核心不认这个键时别的照样读得到。
const VIM_TAG: u64 = TAG_BASE - 12;
/// 新会话开局的级别：常用的那一级、开局只读（核心的配置项）。
const LEVEL_KEYS: [&str; 2] = ["permission.level", "permission.start_read_only"];

impl App {
    /// 头的配置读到了（连上、配置变了）就去读；回应归这里（交回 `true`）。
    pub(super) fn head_settings_update(&mut self, update: &Update) -> bool {
        match update {
            Update::HeadConfig(_) => {
                self.ask_config(ICONS_TAG, &["tui.icons"]);
                self.ask_config(TIMELINE_TAG, &Timeline::KEYS);
                self.ask_config(MASCOT_TAG, &Layout::MASCOT_KEYS);
                self.ask_config(VIM_TAG, &[Layout::KEYS_KEY]);
                let mut keys = crate::theme::choice::Choice::KEYS.to_vec();
                keys.push(crate::theme::choice::SEED_KEY);
                self.ask_config(THEME_TAG, &keys);
                self.ask_config(LEVEL_TAG, &LEVEL_KEYS);
                self.ask_config(CHAT_TAG, &["models.chat"]);
                false
            }
            // 默认的聊天模型：读到了照它要底栏的模型资料（`effort.rs`）。
            Update::Answer { tag, result } if *tag == CHAT_TAG => {
                self.chat_configured(result.as_ref().ok());
                true
            }
            Update::Answer { tag, result } if *tag == ICONS_TAG => {
                if let Ok(got) = result {
                    self.take_icons(got);
                }
                true
            }
            Update::Answer { tag, result } if *tag == TIMELINE_TAG => {
                if let Ok(got) = result {
                    self.config.timeline.apply(got);
                }
                true
            }
            Update::Answer { tag, result } if *tag == MASCOT_TAG => {
                if let Ok(got) = result {
                    self.config.layout.apply_mascot(got);
                }
                true
            }
            Update::Answer { tag, result } if *tag == VIM_TAG => {
                if let Ok(got) = result {
                    self.config.layout.apply_keys(got);
                }
                true
            }
            // 主题那几项（「主题」第 6 条）：读到的盖上，照新的挑一套。
            Update::Answer { tag, result } if *tag == THEME_TAG => {
                if let Ok(got) = result {
                    self.take_theme(got);
                }
                true
            }
            // 新会话开局的级别（「权限级别」第 2 条，2026-10-14）：首页照它画。
            Update::Answer { tag, result } if *tag == LEVEL_TAG => {
                if let Ok(got) = result {
                    self.default_level(got);
                }
                true
            }
            _ => false,
        }
    }

    /// 照读到的终端配置项办（先画后连时照上次记下的，`look.rs`）：图标、时间线、吉祥物的开关、按键模式、主题、开局级别；没有的照旧。
    pub(super) fn apply_head_settings(&mut self, got: &Value) {
        self.take_icons(got);
        self.config.timeline.apply(got);
        self.config.layout.apply_mascot(got);
        self.config.layout.apply_keys(got);
        self.take_theme(got);
        self.default_level(got);
    }

    /// 图标那一套（「图标」第 4 条）。
    fn take_icons(&mut self, got: &Value) {
        if let Some(name) = got["items"]["tui.icons"]["value"].as_str() {
            let name = name.to_string();
            self.use_icons(&name);
        }
    }

    /// 主题那几项（「主题」第 6 条）：读到的盖上，照新的挑一套。
    fn take_theme(&mut self, got: &Value) {
        self.screen.theme_choice.apply(got);
        // 跟随壁纸自选的主色（2026-10-14）：换了要重挑一套。
        self.screen.theme_pick = crate::theme::choice::chosen_seed(got);
        self.screen.theme_seed = self.wanted_seed();
        self.apply_theme();
    }

    /// 读到新会话开局的级别：开局只读的是只读，不然照常用的那一级；认不出的不动。还没开会话、这一回的首页上没按过 Tab
    /// 的，当场照它画。
    fn default_level(&mut self, got: &Value) {
        let value = |key: &str| &got["items"][key]["value"];
        let level = if value("permission.start_read_only") == true {
            Some(crate::core::Level::ReadOnly)
        } else {
            value("permission.level")
                .as_str()
                .and_then(crate::core::Level::parse)
        };
        if let Some(level) = level {
            self.talk.default_level = level;
            self.show_default_level();
        }
    }
}
