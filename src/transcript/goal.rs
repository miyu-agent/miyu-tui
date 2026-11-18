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

    /// 循环到点交进来的那一句（序号 `seq`）落了盘：不照原文画（交进去的是给模型看的英文），画一行「◷ 循环 · 继续」（2026-11-17
    /// 项目主人验收第 3 条，同网页）。和长期目标开的一轮那一行一个样子，开轮时照序号归到那一轮。
    pub(super) fn wake(&mut self, seq: u64, words: &[String; 2]) {
        self.finish_segment();
        let id = self.fresh_id();
        let text = words[1].clone();
        self.entries.push(Entry {
            seq: Some(seq),
            mark: Some(words[0].clone()),
            ..Entry::new(id, Kind::Goal, text)
        });
    }
}
