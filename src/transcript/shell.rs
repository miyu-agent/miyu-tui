//! 你用 `!` 打的命令（蓝图 `tui.md`「人自己跑命令 `!`」第 2、4 条，排期第 21 条）：发出去时画成一条跑命令的话，落了盘照序号认，
//! 跑完了结果挂到那一条下面。

use super::entry::Shell;
use super::{Kind, Transcript};
use crate::core::ShellRan;

impl Transcript {
    /// 你发出去一条命令：画成跑命令的那一条（「┃ ! 命令行」）。
    pub fn shell_user(&mut self, command: String) {
        self.user(command, Vec::new());
        if let Some(entry) = self.entries.last_mut() {
            entry.shell = Some(Shell::default());
        }
    }

    /// 序号是 `seq` 的那句话是跑命令的（补发的、别处发的）。
    pub(super) fn shell_asked(&mut self, seq: u64) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .rev()
            .find(|e| e.kind == Kind::User && e.seq == Some(seq))
        {
            entry.shell.get_or_insert_with(Shell::default);
        }
    }

    /// 跑完了：结果挂到那句话下面。
    pub(super) fn shell_ran(&mut self, ran: ShellRan) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .rev()
            .find(|e| e.kind == Kind::User && e.seq == Some(ran.message))
        {
            entry.shell = Some(Shell { ran: Some(ran) });
        }
    }
}
