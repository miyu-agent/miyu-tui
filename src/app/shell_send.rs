//! 发一条 `!` 打的命令（蓝图 `tui.md`「人自己跑命令 `!`」第 2 条，排期第 21 条）：照普通消息发（开会话、排队、`Ctrl+Enter`
//! 都一样），`session.send` 多 `shell: true`；不能带附件；记进输入历史时前面加 `!`。

use super::App;
use crate::core::Command;
use crate::input::Draft;

impl App {
    /// 回车发出去的是一条命令。
    pub(super) fn submit_shell(&mut self, draft: Draft) {
        // 不能带附件（核心 SH-1 回 `bad_params`）：弹一句，字留在框里。
        if draft
            .blocks
            .iter()
            .any(|b| b.attachment.is_some() || b.path.is_some())
        {
            let note = self.config.text.shell.no_attachments.clone();
            self.hint(note, false);
            self.typing.input.editor.set_draft(draft);
            return;
        }
        // 连不上核心、正在切会话、放不进协议的一行：发不出去，字留在框里（「连核心」第 8 条、「输入框」第 11 条）。
        if !self.reachable() || self.oversized(&draft, false).is_err() {
            self.typing.input.editor.set_draft(draft);
            return;
        }
        let typed = self.typing.input.as_typed(&draft);
        self.typing.input.leave_shell();
        self.leave_gone();
        self.screen.view.follow();
        let command = draft.expand();
        self.talk.unsent = Some(super::Unsent {
            draft: Some(typed.clone()),
            original: None,
            turn: None,
        });
        self.typing.input.remember(typed);
        self.typing.input.forget_cleared();
        self.dream_wake_up();
        self.talk.transcript.shell_user(command.clone());
        self.awaiting_turn();
        self.link.core.send(Command::Send {
            text: command,
            files: Vec::new(),
            shell: true,
            skills: Vec::new(),
            cwd: None,
            pasted: None,
        });
    }
}
