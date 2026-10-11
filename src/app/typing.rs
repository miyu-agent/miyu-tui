//! 输入（蓝图 `app.md` 的 `typing` 块）：输入框和它上面的列表（斜杠命令、`@` 文件、输入历史），焦点、两下 Esc 的
//! 时限、交给编辑器写的那句、贴图暂存在哪、编辑器、输入法、空着时的提示、框下面那一行的临时提示。

use std::time::Instant;

use super::{Notice, compose};
use crate::focus::Focus;
use crate::history::History;
use crate::input::InputBox;
use crate::menu::Menu;
use crate::tips::Tips;

/// 输入。
pub struct Typing {
    /// 输入框。
    pub input: InputBox,
    /// 斜杠命令列表。
    pub menu: Menu,
    /// `@` 文件列表。
    pub mention: crate::mention::Mention,
    /// 输入历史列表（`Ctrl+R`）。
    pub history: History,
    /// 输入历史放好了（照 `package.list` 定了在哪、读进来了）。
    pub history_placed: bool,
    /// 9-1 以前输入历史的老位置：拿不到包的状态目录的照它，拿到了的搬过去（`packages.rs`）。
    pub old_history: Option<std::path::PathBuf>,
    /// 焦点在哪：输入框、框下面那一行的按钮、子代理状态行。
    pub focus: Focus,
    /// 在回答时第一下 `Esc` 的时刻：时限里再按一下才打断。
    pub esc_at: Option<Instant>,
    /// `Ctrl+G` 正在编辑器里写的那句（`compose.rs`）。
    pub composing: Option<compose::Composing>,
    /// `Ctrl+V` 贴的截图暂存在哪（「输入框」第 12 条）；找不到缓存目录的是 `None`，贴不了图。
    pub staging: Option<crate::clipboard::Staging>,
    /// 本机的文本文件用哪个编辑器开（`$VISUAL`、`$EDITOR`）；没有的交给系统（`editor.rs`）。
    pub editor: Option<String>,
    /// 点了本机的文本文件、按了 `Ctrl+G`：主循环让出终端给编辑器，编辑器和文件。
    pub edit: Option<(String, std::path::PathBuf)>,
    /// 输入法跟着打字状态切（`ime.rs`）。
    pub ime: crate::ime::Ime,
    /// 输入框空着时写哪条提示。
    pub tips: Tips,
    /// 框下面那一行的临时提示。
    pub notice: Option<Notice>,
}
