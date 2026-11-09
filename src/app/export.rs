//! `/export`：把正在看的会话导出来（排期第 36 条，核心 SE-1 的 `session.export`；会话导出调研第五节第 5 条；蓝图 `tui.md`
//! 「导出 `/export`」）。先问给谁用（完整存档、用于给别的 agent 继续、用于网页端聊天对话、仅导出对话为 Markdown 文件），三种 Markdown 一打开就各要一份放进回应的，
//! 右边写大小和 token 估算；再问复制还是存成文件（存档只能存文件）。存成文件的写在终端所在的目录，名字带会话的短编号和用途，
//! 已经有了的接 `-2`。

use std::path::{Path, PathBuf};

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::{App, Panel};
use crate::core::{Command, Update};
use crate::oobe::asker::TAG_BASE;

/// 三种 Markdown 估算的那三条（照 [`Use::ALL`] 的先后，存档不估）。
const PREVIEW_TAG: u64 = TAG_BASE - 30;
/// 存成文件的那一条。
const SAVE_TAG: u64 = TAG_BASE - 34;

/// 给谁用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    /// 存档：原始包，只能存文件。
    Bundle,
    /// 给别的 agent（Markdown 的 `agent`）。
    Agent,
    /// 贴进网页 chat（`chat`）。
    Chat,
    /// 自己看（`read`）。
    Read,
}

impl Use {
    /// 框里的先后。
    pub const ALL: [Use; 4] = [Use::Bundle, Use::Agent, Use::Chat, Use::Read];

    /// 第几行。
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|u| *u == self).unwrap_or(0)
    }

    /// Markdown 的预设；存档是 `None`。
    fn preset(self) -> Option<&'static str> {
        match self {
            Use::Bundle => None,
            Use::Agent => Some("agent"),
            Use::Chat => Some("chat"),
            Use::Read => Some("read"),
        }
    }
}

/// 一种 Markdown 估得怎样。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Preview {
    /// 还在估。
    Waiting,
    /// 估好了：多少字节、多少 token；放进回应的全文（复制用），太大放不进的是 `None`（只能存文件）。
    Ready {
        bytes: u64,
        tokens: u64,
        text: Option<String>,
    },
    /// 核心拒了：它的原话。
    Failed(String),
}

/// 开着的框。
#[derive(Debug, Clone)]
pub struct ExportBox {
    /// 导哪个会话。
    pub session: String,
    /// 存在哪个目录（终端所在的）。
    dir: PathBuf,
    /// 三种 Markdown 估的（`agent`、`chat`、`read`）。
    previews: [Preview; 3],
    /// 第一步选好了：第二步问复制还是存文件。
    pub chosen: Option<Use>,
}

impl ExportBox {
    /// 这种用途估的；存档没有，交回还在估。
    pub fn preview(&self, used: Use) -> &Preview {
        match used {
            Use::Bundle => &Preview::Waiting,
            _ => &self.previews[used.index() - 1],
        }
    }

    /// 存成文件时写的名字（只写名字，目录是终端所在的）。
    pub fn file_name(&self, used: Use) -> String {
        self.target(used)
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
    }

    /// 存成文件写到哪：`miyu-<短编号>-<用途>.md`、存档 `miyu-<短编号>.zip`；已经有了的接 `-2`、`-3`。
    fn target(&self, used: Use) -> PathBuf {
        let short = crate::session_list::short(&self.session);
        let (stem, ext) = match used.preset() {
            Some(preset) => (format!("miyu-{short}-{preset}"), "md"),
            None => (format!("miyu-{short}"), "zip"),
        };
        free_name(&self.dir, &stem, ext)
    }
}

/// 存到哪个目录：终端所在的目录。`shown` 是侧边栏写的那份，家目录写成了 `~`，展开回去（核心只收绝对路径；2026-11-05
/// 验收：工作目录在家目录下时存文件回「参数错误」）。
fn export_dir(shown: &str, home: Option<&Path>) -> PathBuf {
    match home {
        // 正好在家目录：侧边栏写的是单独一个 `~`。
        Some(home) if shown == "~" => home.to_path_buf(),
        _ => crate::local::expand_home(shown.into(), home),
    }
}

/// `dir` 里还没有的 `<stem>.<ext>`、`<stem>-2.<ext>`……（Markdown 连同附件文件夹 `<stem>_files` 也算）。
fn free_name(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    (1..)
        .map(|n| match n {
            1 => stem.to_string(),
            n => format!("{stem}-{n}"),
        })
        .map(|name| dir.join(format!("{name}.{ext}")))
        .find(|path| !path.exists())
        .unwrap_or_else(|| dir.join(format!("{stem}.{ext}")))
}

impl App {
    /// 打开框：三种 Markdown 当场各要一份放进回应的，估大小和 token。还没开会话的提示一句。
    pub(super) fn open_export(&mut self) {
        let Some(session) = self.talk.transcript.session.clone() else {
            self.hint(self.config.text.export.no_session.clone(), false);
            return;
        };
        for (i, used) in Use::ALL[1..].iter().enumerate() {
            self.link.core.send(Command::Ask {
                tag: PREVIEW_TAG + i as u64,
                method: "session.export",
                params: json!({"session": session, "format": "markdown", "for": used.preset(), "to": "inline"}),
            });
        }
        self.boxes.export = Some(ExportBox {
            session,
            dir: export_dir(&self.cwd, std::env::home_dir().as_deref()),
            previews: std::array::from_fn(|_| Preview::Waiting),
            chosen: None,
        });
        self.boxes.panel = Some(Panel::Export { selected: 0 });
    }

    /// 框开着时的按键：`↑` `↓` 选，`Enter` 选好，`Esc` 第二步回第一步、第一步关。
    pub(super) fn export_key(&mut self, selected: usize, key: KeyEvent) {
        let Some(open) = self.boxes.export.clone() else {
            self.boxes.panel = None;
            return;
        };
        let last = if open.chosen.is_some() { 1 } else { 3 };
        let to = |selected| Some(Panel::Export { selected });
        match key.code {
            KeyCode::Up => self.boxes.panel = to(selected.saturating_sub(1)),
            KeyCode::Down => self.boxes.panel = to((selected + 1).min(last)),
            KeyCode::Esc if open.chosen.is_some() => {
                let back = open.chosen.map_or(0, Use::index);
                self.set_chosen(None);
                self.boxes.panel = to(back);
            }
            KeyCode::Esc => self.close_export(),
            KeyCode::Enter => self.export_enter(&open, selected),
            _ => {}
        }
    }

    fn set_chosen(&mut self, chosen: Option<Use>) {
        if let Some(open) = self.boxes.export.as_mut() {
            open.chosen = chosen;
        }
    }

    fn close_export(&mut self) {
        self.boxes.panel = None;
        self.boxes.export = None;
    }

    /// `Enter`：第一步选了存档的直接存，选了 Markdown 的进第二步；第二步复制（估好了、放得进回应的）或者存成文件。
    fn export_enter(&mut self, open: &ExportBox, selected: usize) {
        let Some(used) = open.chosen else {
            let used = Use::ALL[selected.min(3)];
            if used == Use::Bundle {
                return self.export_save(open, used);
            }
            self.set_chosen(Some(used));
            self.boxes.panel = Some(Panel::Export { selected: 0 });
            return;
        };
        match (selected, open.preview(used)) {
            (
                0,
                Preview::Ready {
                    tokens,
                    text: Some(text),
                    ..
                },
            ) => {
                let (text, tokens) = (text.clone(), *tokens);
                self.close_export();
                self.copy_export(&text, tokens);
            }
            // 还在估、太大、拒了的不能复制：不动。
            (0, _) => {}
            _ => self.export_save(open, used),
        }
    }

    /// 复制：进剪贴板，提示「已复制 · 约 N tokens」（同全屏别处的复制，读不成的照原话说）。
    fn copy_export(&mut self, text: &str, tokens: u64) {
        match crate::clipboard::copy(text) {
            Ok(()) => {
                let note = self
                    .config
                    .text
                    .export
                    .copied
                    .replace("{tokens}", &crate::meter::short(tokens));
                self.hint(note, true);
            }
            Err(e) => {
                let words = &self.config.text;
                self.hint(words.copy_failed.replace("{reason}", &e.to_string()), false);
            }
        }
    }

    /// 存成文件：照用途写参数，交给核心写；回来了提示写到哪、多大（[`App::export_update`]）。
    fn export_save(&mut self, open: &ExportBox, used: Use) {
        let path = open.target(used).display().to_string();
        let params = match used.preset() {
            Some(preset) => {
                json!({"session": open.session, "format": "markdown", "for": preset, "to": {"path": path}})
            }
            None => json!({"session": open.session, "format": "bundle", "to": {"path": path}}),
        };
        self.link.core.send(Command::Ask {
            tag: SAVE_TAG,
            method: "session.export",
            params,
        });
        self.close_export();
    }

    /// 导出的回应归这里交回 `true`：估的记下（框关了的不要），存的提示写到哪，拒了的弹核心的原话。
    pub(super) fn export_update(&mut self, update: &Update) -> bool {
        let Update::Answer { tag, result } = update else {
            return false;
        };
        if (PREVIEW_TAG..PREVIEW_TAG + 3).contains(tag) {
            let i = usize::try_from(tag - PREVIEW_TAG).unwrap_or(0);
            if let Some(open) = self.boxes.export.as_mut() {
                open.previews[i] = preview(result);
            }
            return true;
        }
        if *tag != SAVE_TAG {
            return false;
        }
        match result {
            Ok(saved) => {
                let path = saved["path"].as_str().unwrap_or_default();
                let note = self
                    .config
                    .text
                    .export
                    .saved
                    .replace("{path}", &crate::local::home_short(path))
                    .replace(
                        "{size}",
                        &crate::drawer::size(saved["bytes"].as_u64().unwrap_or(0)),
                    );
                self.hint(note, true);
            }
            Err(refused) => self.hint(refused.message.clone(), false),
        }
        true
    }
}

/// 估的那一条的回应：放得进回应的带全文；超过 512 KiB 的（`export_too_big`）只记得太大；别的拒绝照核心的原话。
fn preview(result: &Result<Value, crate::core::Refusal>) -> Preview {
    match result {
        Ok(got) => Preview::Ready {
            bytes: got["bytes"].as_u64().unwrap_or(0),
            tokens: got["tokens"].as_u64().unwrap_or(0),
            text: got["text"].as_str().map(str::to_string),
        },
        Err(refused) if refused.reason.as_deref() == Some("export_too_big") => Preview::Ready {
            bytes: 0,
            tokens: 0,
            text: None,
        },
        Err(refused) => Preview::Failed(refused.message.clone()),
    }
}

#[cfg(test)]
mod tests;
