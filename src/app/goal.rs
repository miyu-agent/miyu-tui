//! `/goal`（蓝图 `tui.md`「长期目标」第 1、3 条）：交给核心办；还没开的会话里设目标先开会话，看、停、改当场答「无长期目标」。

use super::App;
use crate::core::Command;

impl App {
    /// `/goal`，`words` 是后面的字。
    pub(super) fn goal_command(&mut self, words: Option<&str>) {
        let words = words.unwrap_or_default().trim();
        if self.not_opened() && !sets_goal(words) {
            let note = self.config.text.goal.none.clone();
            self.hint(note, false);
            return;
        }
        // 会话不在了（「连核心」第 7 条）：设目标和说下一句话一样，开新会话。
        self.leave_gone();
        self.link.core.send(Command::Goal(words.to_string()));
    }
}

/// `/goal` 后面的字是不是设一个目标，照核心的读法（核心 `commands/goal.rs`）：没字的是看，单独的 `pause`、`resume`、
/// `clear` 是子命令，`edit` 打头的是改；别的整句当目标（前面可以写 `--rounds N`）。
fn sets_goal(words: &str) -> bool {
    let verb = words.split_whitespace().next().unwrap_or_default();
    let alone = words == verb;
    !(words.is_empty() || verb == "edit" || alone && matches!(verb, "pause" | "resume" | "clear"))
}

/// `/goal` 看进度的回执「原文 · 在哪一步 · 第几轮」（核心 `commands/goal-is`）拆成原文和后面那一截（连着前面的 ` · `）：
/// 提示框放不下时截原文，后面整个留着（「长期目标」第 1 条，2026-10-12 项目主人定）。没有两个 ` · ` 的整句是原文。
pub(super) fn split_progress(said: &str) -> (&str, &str) {
    let mut cuts = said.rmatch_indices(" · ").map(|(at, _)| at);
    match (cuts.next(), cuts.next()) {
        (Some(_), Some(at)) => said.split_at(at),
        _ => (said, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::{sets_goal, split_progress};

    #[test]
    fn the_progress_receipt_splits_off_its_state_and_rounds() {
        // 核心 `commands/goal-is`：「原文 · 在哪一步 · 第几轮」，原文里也可以有 ` · `。
        assert_eq!(
            split_progress("写完 · 测试 · 进行中 · 第 2/5 轮"),
            ("写完 · 测试", " · 进行中 · 第 2/5 轮")
        );
        assert_eq!(split_progress("已暂停长期目标"), ("已暂停长期目标", ""));
    }

    #[test]
    fn only_a_goal_to_set_opens_a_session() {
        for words in [
            "写完测试",
            "--rounds 5 写完测试",
            "pause 一下再说",
            "clear the backlog",
        ] {
            assert!(sets_goal(words), "{words}");
        }
        for words in ["", "pause", "resume", "clear", "edit 换个目标", "edit"] {
            assert!(!sets_goal(words), "{words}");
        }
    }
}
