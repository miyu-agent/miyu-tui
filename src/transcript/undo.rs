//! 撤销说明那一行（蓝图 `tui.md`「正文」第 5、6 条）：看着撤的照撤销的回应画；补发时没有回应，照补发来的事件画
//! （`core/replay.rs` 的 `Push::UndoLine`、`UndoGone`）。

use super::{Kind, Transcript};
use crate::core::Report;

impl Transcript {
    /// 撤掉、恢复的那几轮的条目藏起来、显示回来（从 `mod.rs` 挪过来）。
    pub(super) fn hide(&mut self, turns: &[u64], hidden: bool) {
        // 别处来的话撤销不带走它（「别处来的话」第 3 条）。
        for entry in self.entries.iter_mut().filter(|e| e.from.is_none()) {
            if entry
                .turn
                .or(entry.covers)
                .is_some_and(|t| turns.contains(&t))
            {
                entry.hidden = hidden;
            }
        }
    }

    /// 最后一条记在在跑的那一轮名下（抽屉了结的结果：回答了的提问那一块、「不允许」那一行，`app/drawer.rs` 的
    /// `report_into`）：撤掉这一轮时跟着藏、恢复时跟着回来（[`Transcript::hide`]）。没在跑的不记。原来这两样不带轮次，
    /// 撤销以后照样留在正文里（同 LiteMiyu `dd889d8`）。
    pub fn tie_last_to_turn(&mut self) {
        let turn = self.turn;
        if let Some(last) = self.entries.last_mut().filter(|_| turn.is_some()) {
            last.turn = turn;
        }
    }

    /// 画一行撤销说明：全文照这一次撤掉的第一轮里你说的话，没有的照回应里的第一行。
    pub(super) fn undo_line(&mut self, report: Report) {
        let said = self
            .entries
            .iter()
            .find(|e| e.kind == Kind::User && e.turn.is_some_and(|t| self.reverted.contains(&t)))
            .map(|e| e.text.clone())
            .or_else(|| report.said.clone())
            .unwrap_or_default();
        self.note(Kind::Undo, said);
        if let Some(last) = self.entries.last_mut() {
            last.undo = Some(report);
        }
    }

    /// 恢复、重做：去掉最近那一行撤销说明，不另写一句。
    pub(super) fn undo_gone(&mut self) {
        if let Some(i) = self.entries.iter().rposition(|e| e.kind == Kind::Undo) {
            self.entries.remove(i);
            self.removed(i);
        }
    }
}
