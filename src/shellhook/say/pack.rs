//! 主题包（同终端界面「主题」第 3、5 条，`app/theme_pick.rs`）：读 `tui.look`；是一个主题包的编号就照 `package.list` 找它的
//! `theme.json`、`package.file` 读来、盖在默认主题上换上（`sheet::set`，记号、框线、修饰），`layout`、`timeline` 照它重读，
//! 时间线的配置项再读一遍盖上。没写的、是默认主题包的照编进程序的默认主题；读不成的照默认主题，shell 里不弹提示。
//! 每句现取（同配色）：换了主题包，下一句就照新的画。

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use crate::config::Config;
use crate::core::{Command, Core, Update};

/// 读 `tui.look` 的请求编号（说话那一套用个位数）。
const SETTING_TAG: u64 = 4;
/// 找主题包的文件（`package.list`）。
const LIST_TAG: u64 = 5;
/// 读主题文件（`package.file`）。
const FILE_TAG: u64 = 6;

/// 读到哪了。
#[derive(Debug, Default)]
pub struct Pack {
    /// 在读的那个包。
    loading: Option<String>,
}

/// 换上一整份主题以后要做的：时间线的配置项再读一遍盖上（`retheme` 照主题重读了 `timeline`）。
pub enum Applied {
    /// 不是这里的回应。
    No,
    /// 办了。
    Yes,
    /// 换上了：再读时间线的配置项。
    Retimeline,
}

impl Pack {
    /// 头的配置到了（连上时）：问 `tui.look`。
    pub fn ask(core: &Core) {
        core.send(Command::Ask {
            tag: SETTING_TAG,
            method: "config.get",
            params: json!({ "keys": [crate::settings::pages::LOOK] }),
            system: false,
        });
    }

    /// 几个请求的回应归这里。
    pub fn update(&mut self, update: &Update, config: &mut Config, core: &Core) -> Applied {
        let Update::Answer { tag, result } = update else {
            return Applied::No;
        };
        match *tag {
            SETTING_TAG => {
                let id = result.as_ref().ok().and_then(|got| {
                    got["items"][crate::settings::pages::LOOK]["value"]
                        .as_str()
                        .filter(|id| *id != crate::sheet::DEFAULT_PACKAGE)
                        .map(str::to_string)
                });
                match id {
                    // 默认主题：编进程序的，起来时就是它。
                    None => Applied::Yes,
                    Some(id) => {
                        self.loading = Some(id);
                        core.send(Command::Ask {
                            tag: LIST_TAG,
                            method: "package.list",
                            params: json!({}),
                            system: false,
                        });
                        Applied::Yes
                    }
                }
            }
            LIST_TAG => {
                let file = result.as_ref().ok().and_then(|got| self.file_of(got));
                match (self.loading.clone(), file) {
                    (Some(id), Some(path)) => core.send(Command::Ask {
                        tag: FILE_TAG,
                        method: "package.file",
                        params: json!({ "package": id, "path": path }),
                        system: false,
                    }),
                    _ => self.loading = None,
                }
                Applied::Yes
            }
            FILE_TAG => {
                if self.loading.take().is_none() {
                    return Applied::Yes;
                }
                let pack = result
                    .as_ref()
                    .ok()
                    .and_then(|got| STANDARD.decode(got["data"].as_str()?).ok())
                    .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
                let Some(pack) = pack else {
                    return Applied::Yes;
                };
                let whole = crate::sheet::merged(&pack);
                // 读不成的照默认主题（起来时就是它），不另外办。
                match crate::sheet::set(&whole).and_then(|()| config.retheme(&whole)) {
                    Ok(()) => Applied::Retimeline,
                    Err(_) => Applied::Yes,
                }
            }
            _ => Applied::No,
        }
    }

    /// `package.list` 里在读的那个包的主题文件：得是读成了的主题包（同 `app/theme_pick.rs`）。
    fn file_of(&self, got: &Value) -> Option<String> {
        let id = self.loading.as_deref()?;
        got["packages"]
            .as_array()?
            .iter()
            .find(|p| p["package"] == id && p["kind"] == "theme")
            .and_then(|p| p["theme"]["file"].as_str())
            .map(str::to_string)
    }
}
