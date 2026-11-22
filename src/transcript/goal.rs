//! 长期目标开的一轮（蓝图 `tui.md`「长期目标」第 4 条）：开它的那句是给她看的续轮消息，不照原文画，画成暗色一行
//! 「● 长期目标 · 第 N 轮」。

use super::{Entry, Kind, Transcript};
use crate::config::GoalTexts;

impl Transcript {
    /// 长期目标开一轮的那一句（序号 `seq`，第 `round` 轮）落了盘：画一行，序号记上，开轮时照它归到那一轮（`turn.rs`）。
    pub(super) fn goal_round(&mut self, seq: u64, round: Option<u32>, texts: &GoalTexts) {
        let text = match round {
            Some(round) => texts.round.replace("{round}", &round.to_string()),
            None => texts.some_round.clone(),
        };
        self.finish_segment();
        let id = self.fresh_id();
        self.entries.push(Entry {
            seq: Some(seq),
            mark: Some(texts.mark.clone()),
            ..Entry::new(id, Kind::Goal, text)
        });
    }

    /// 循环、AI 挂的定时触发到点交进来的那一句（序号 `seq`）落了盘：照别处来的话画（`ui/foreign_rows.rs`），左边灰竖线、暗字
    /// 「来处 · 那一句」，点开看全文；开轮时照序号归到那一轮（2026-11-22 主人：「原先不是类似用户消息的形式吗？tui 的话左侧
    /// 竖线是灰色的」，照 LiteMiyu）。
    pub(super) fn wake(&mut self, seq: u64, from: String, text: String) {
        self.finish_segment();
        let id = self.fresh_id();
        self.entries.push(Entry {
            seq: Some(seq),
            from: Some(from),
            ..Entry::new(id, Kind::User, text)
        });
    }
}
