//! 供应商侧工具（模型在供应商那边自己跑的搜索、读网页，事件里的 `hosted_tool`，核心 HT-1）：核心不执行，没有调用编号、
//! 没有 `tool.result`，结果跟着那一块来（`outcome`）。照核心 `view.md`「供应商侧工具」定状态：结果到了是成了（供应商说
//! 失败了的是出错）；回复落了盘还没有结果的，回复被打断了是取消，不然是成了。

use serde_json::Value;

use super::steps::{Step, StepKind, ToolState};
use super::{Slot, Transcript};
use crate::core::{Outcome, ToolStatus};

impl Transcript {
    /// 第 `index` 块供应商侧工具的结果到了：参数读出来，停表，写状态、字。
    pub(super) fn hosted_outcome(&mut self, index: u64, outcome: Outcome) {
        let Some(Slot::Step(i, j)) = self.blocks.get(&index).copied() else {
            return;
        };
        let now = self.clock.now();
        let Some(step) = self.step_mut((i, j)).filter(|s| s.hosted.is_some()) else {
            return;
        };
        let status = if outcome.failed {
            ToolStatus::Error
        } else {
            ToolStatus::Ok
        };
        settle(step, status, now);
        if let StepKind::Tool { output, .. } = &mut step.kind {
            *output = outcome.text;
        }
    }

    /// 回复落了盘：这一轮还没结果的供应商侧工具定下来。
    pub(super) fn settle_hosted(&mut self, interrupted: bool) {
        let now = self.clock.now();
        let status = if interrupted {
            ToolStatus::Cancelled
        } else {
            ToolStatus::Ok
        };
        let turn = self.turn;
        let segments = self
            .entries
            .iter_mut()
            .rev()
            .filter(|e| e.turn == turn)
            .filter_map(|e| e.segment.as_mut());
        for step in segments.flat_map(|s| s.steps.iter_mut()) {
            if step.hosted.is_some() && step.busy() {
                settle(step, status, now);
            }
        }
    }
}

/// 定成 `status`：参数还没读的读出来，停表。
fn settle(step: &mut Step, status: ToolStatus, now: std::time::Instant) {
    step.stop_at(now);
    if let StepKind::Tool {
        args,
        parsed,
        state,
        ..
    } = &mut step.kind
    {
        if parsed.is_null() {
            *parsed = serde_json::from_str(args).unwrap_or(Value::Null);
        }
        *state = ToolState::Done(status);
    }
}
