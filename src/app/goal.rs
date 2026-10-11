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

#[cfg(test)]
mod tests {
    use super::sets_goal;

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
