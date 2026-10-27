//! 工具的结果、在跑时的进度（从 `mod.rs` 拆出来，2026-10-26）：照调用编号找回那一步，结果停表、写状态和字，进度写最新
//! 的一行（蓝图 `tui.md`「时间线」第 8、8a 条）。

use super::Transcript;
use super::steps::{StepKind, ToolState};
use crate::core::Push;

impl Transcript {
    /// 一件工具的结果：停表，写状态、字、结果那一句；进度去掉。
    pub(super) fn tool_result(&mut self, push: Push) {
        let Push::ToolResult {
            call_id,
            status,
            text,
            said: result_said,
            display,
            sent,
            detail,
        } = push
        else {
            return;
        };
        self.progress.over(&call_id);
        let at = self.calls.get(&call_id).copied();
        let now = self.clock.now();
        if let Some(step) = at.and_then(|at| self.step_mut(at)) {
            step.stop_at(now);
            step.display = display;
            step.sent = sent;
            step.detail = detail;
            step.progress = None;
            if let StepKind::Tool {
                state,
                output,
                said,
                ..
            } = &mut step.kind
            {
                *state = ToolState::Done(status);
                *output = text;
                *said = result_said;
            }
        }
    }

    /// 在跑的工具推来一段输出：收全了一行进度的，写进那一步。
    pub(super) fn tool_progress(&mut self, call_id: &str, text: &str) {
        let Some(line) = self.progress.feed(call_id, text) else {
            return;
        };
        let at = self.calls.get(call_id).copied();
        if let Some(step) = at.and_then(|at| self.step_mut(at)) {
            step.progress = Some(line);
        }
    }
}
