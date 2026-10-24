//! 换主题（蓝图 `tui.md`「主题」第 3、5 条）：连上时、配置变了时读 `tui.look`；是一个主题包的编号就照 `package.list` 找它的
//! `theme.json`、`package.file` 读来、盖在默认主题上换上；没写的、是默认主题包的照编进程序的默认主题；包没了、读不成的照默认
//! 主题画，弹一句。照吉祥物包的那一套（`mascot_pick.rs`）。

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use super::App;
use crate::core::{Command, Update};
use crate::oobe::asker::TAG_BASE;

/// 读 `tui.look` 的请求编号（`TAG_BASE - 12` 留给按键模式）。
const SETTING_TAG: u64 = TAG_BASE - 13;
/// 找主题包的文件的请求编号（`package.list`）。
const LIST_TAG: u64 = TAG_BASE - 14;
/// 读主题文件的请求编号（`package.file`）。
const FILE_TAG: u64 = TAG_BASE - 15;

/// 换主题这件事的状态。
#[derive(Debug, Default)]
pub struct ThemePick {
    /// 在读的那个包。
    loading: Option<String>,
    /// 现在画着的包；默认主题是 `None`。
    shown: Option<String>,
}

impl App {
    /// 连上、配置变了去读 `tui.look`；几个请求的回应归这里（交回 `true`）。
    pub(super) fn theme_pick_update(&mut self, update: &Update) -> bool {
        match update {
            Update::HeadConfig(_) => {
                self.ask_config(SETTING_TAG, &[crate::settings::pages::LOOK]);
                false
            }
            Update::Answer { tag, result } if *tag == SETTING_TAG => {
                let id = result.as_ref().ok().and_then(|got| {
                    got["items"][crate::settings::pages::LOOK]["value"]
                        .as_str()
                        .filter(|id| *id != crate::sheet::DEFAULT_PACKAGE)
                        .map(str::to_string)
                });
                self.look_chosen(id);
                true
            }
            Update::Answer { tag, result } if *tag == LIST_TAG => {
                let file = result.as_ref().ok().and_then(|got| self.theme_file_of(got));
                match (self.media.theme_pick.loading.clone(), file) {
                    (Some(id), Some(path)) => {
                        let params = json!({"package": id, "path": path});
                        self.theme_ask(FILE_TAG, "package.file", params);
                    }
                    (Some(_), None) => self.look_failed(),
                    (None, _) => {}
                }
                true
            }
            Update::Answer { tag, result } if *tag == FILE_TAG => {
                let Some(id) = self.media.theme_pick.loading.take() else {
                    return true;
                };
                let pack = result
                    .as_ref()
                    .ok()
                    .and_then(|got| STANDARD.decode(got["data"].as_str()?).ok())
                    .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
                match pack {
                    Some(pack) => self.put_look(&crate::sheet::merged(&pack), Some(id)),
                    None => self.look_failed(),
                }
                true
            }
            _ => false,
        }
    }

    /// 配置里选的：没写的换回默认主题；和画着的一样的不动；别的去找它的文件。
    fn look_chosen(&mut self, id: Option<String>) {
        let pick = &mut self.media.theme_pick;
        if id == pick.shown && pick.loading.is_none() {
            return;
        }
        match id {
            None => {
                pick.loading = None;
                self.put_look(&crate::sheet::default_value(), None);
            }
            Some(id) => {
                pick.loading = Some(id);
                self.theme_ask(LIST_TAG, "package.list", json!({}));
            }
        }
    }

    /// `package.list` 里在读的那个包的主题文件：得是读成了的主题包。
    fn theme_file_of(&self, got: &Value) -> Option<String> {
        let id = self.media.theme_pick.loading.as_deref()?;
        got["packages"]
            .as_array()?
            .iter()
            .find(|p| p["package"] == id && p["kind"] == "theme")
            .and_then(|p| p["theme"]["file"].as_str())
            .map(str::to_string)
    }

    /// 换上一整份主题：表换了、`layout`、`timeline` 照它重读，终端自己的配置项盖过的那几格再盖一遍。读不成的照默认主题。
    fn put_look(&mut self, whole: &Value, id: Option<String>) {
        let applied = crate::sheet::set(whole).and_then(|()| self.config.retheme(whole));
        if applied.is_err() {
            return self.look_failed();
        }
        self.note_theme(id.is_some().then_some(whole));
        self.media.theme_pick.shown = id;
        let items = Value::Object(self.link.look.items.clone());
        self.apply_layout_settings(&json!({ "items": items }));
    }

    /// 读不成：照默认主题画，弹一句。
    fn look_failed(&mut self) {
        self.media.theme_pick.loading = None;
        self.media.theme_pick.shown = None;
        let default = crate::sheet::default_value();
        let restored = crate::sheet::set(&default).and_then(|()| self.config.retheme(&default));
        if restored.is_ok() {
            let items = Value::Object(self.link.look.items.clone());
            self.apply_layout_settings(&json!({ "items": items }));
        }
        self.note_theme(None);
        let text = self.config.text.look_failed.clone();
        self.hint(text, false);
    }

    /// 发一个请求，回应照编号回来。
    fn theme_ask(&mut self, tag: u64, method: &'static str, params: Value) {
        self.link.core.send(Command::Ask {
            system: false,
            tag,
            method,
            params,
        });
    }
}
