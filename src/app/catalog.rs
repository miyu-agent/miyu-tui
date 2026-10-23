//! 核心认的斜杠命令（蓝图 `tui.md`「斜杠命令列表」第 6 条，核心 O-6 补、X-14）：命令模板、扩展声明的、核心内置而终端
//! 自己没列的。连上、打开命令列表、换界面语言时问一次 `command.catalog`（带着正在看的会话，只列这个会话里能用的），
//! 接在终端自己的命令后面；打了整行交给 `command.run`。终端不认的 `/名字` 也交给核心，由它说为什么不行；被拒的弹它的原话，
//! 打的那一行放回输入框。

use super::App;

/// 打的那一行不放回输入框的原因码：当时的状态不对的（2026-10-20 项目主人报：没在整理时打 `/dream stop`，回车后又出现在
/// 输入框里），没有这个命令的（2026-10-23 项目主人：「输入不存在的命令不应该把命令再回填到输入框」）。再发一遍也一样。
const STATE_REFUSALS: &[&str] = &["not_dreaming", "unknown_command"];
use crate::core::{Command, CoreCommand};
use crate::input::Draft;

impl App {
    /// 问核心认哪些斜杠命令。
    pub(super) fn ask_catalog(&mut self) {
        let session = self.talk.transcript.session.clone();
        self.link.core.send(Command::Catalog(session));
    }

    /// 核心的清单到了：接在终端自己的命令后面（同名的照终端自己的）。
    pub(super) fn catalog_arrived(&mut self, listed: &[CoreCommand]) {
        self.config.commands.take_core(listed);
    }

    /// 一行斜杠命令交给核心（核心清单里的、终端不认的名字）：没连上的字留在输入框；记下那一行，被拒了放回去。
    pub(super) fn slash(&mut self, line: &str) {
        let draft = Draft::plain(line);
        if !self.reachable() {
            self.typing.input.editor.set_draft(draft);
            return;
        }
        // 会话不在了（「连核心」第 7 条）：和说下一句话一样，开新会话。
        self.leave_gone();
        self.typing.slash_draft = Some(draft);
        // 终端在看着的那一趟整理：`/dream stop` 照 Esc 叫停（`dream.rs`）；没在看的照常交给核心，由它停或者说为什么不行。
        if crate::dream::asks_to_stop(line) && self.dream_stop() {
            self.typing.slash_draft = None;
            return;
        }
        self.dream_start(line);
        self.link.core.send(Command::Slash(line.to_string()));
    }

    /// 交给核心的那一行办了（有回执的照常弹）：不用再放回去。
    pub(super) fn slash_ran(&mut self) {
        self.typing.slash_draft = None;
    }

    /// 交给核心的那一行被拒：弹核心的原话（去掉句末的句号；扩展的原话在下一行），输入框空着的放回打的那一行；当时的状态不对
    /// 的（[`STATE_REFUSALS`]）不放回，再发一遍也一样。
    pub(super) fn slash_refused(&mut self, message: &str, reason: Option<&str>) {
        let said = message.trim_end().trim_end_matches(['。', '.']);
        if !said.is_empty() {
            self.hint(said.to_string(), false);
        }
        let draft = self.typing.slash_draft.take();
        if reason.is_some_and(|r| STATE_REFUSALS.contains(&r)) {
            return;
        }
        if let Some(draft) = draft
            && self.typing.input.editor.is_empty()
        {
            self.typing.input.editor.set_draft(draft);
        }
    }
}
