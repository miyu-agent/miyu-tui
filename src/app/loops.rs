//! 唤醒表里的循环变了（蓝图 `tui.md`「循环」第 1 条）：改手上那一份；看着的会话上的循环这一次只记下、没开轮（`last.result`
//! 是 `held`），底下提示一句（ACC-1121-#1，2026-11-20 主人本机：撤销了一轮、还能恢复时打的 `/loop` 一直不跑，界面上什么都
//! 没有）。`/loop` 收下以后核心马上交第一次，那一次核心自己的回执已经说了循环先不跑（核心 FX-12），这里不再另写；之后到点、
//! 没有回执的那几次照旧写（2026-11-21 core1 定）。

use std::collections::HashSet;

use super::App;
use crate::core::LoopChange;

/// 打了 `/loop`、回执还没到的时候新出现的循环：它交的第一次由核心的回执说，不另写（`Link::loops_sent`）。
#[derive(Debug, Default)]
pub struct Sent {
    /// 打了 `/loop`，回执还没到。
    pub waiting: bool,
    /// 那时新出现的循环，第一次还没交。
    pub fresh: HashSet<String>,
}

impl App {
    /// 交给核心的那一行是 `/loop`：回执到以前新出现的循环，第一次不另写（`catalog.rs` 交的时候调）。
    pub(super) fn loop_sent(&mut self, line: &str) {
        let name = line.split_whitespace().next();
        if name == Some("/loop") {
            self.link.loops_sent.waiting = true;
        }
    }

    /// 命令的回执：`/loop` 的到了，之后新出现的循环不算它的。
    pub(super) fn receipt(&mut self, command: &str, text: String, tail: String) {
        if command == "loop" {
            self.link.loops_sent.waiting = false;
        }
        self.hint_keeping(text, tail, false);
    }

    /// 唤醒表推来一项变化。
    pub(super) fn loops_changed(&mut self, change: LoopChange) {
        let viewing = self.talk.transcript.session.as_deref();
        let held = crate::core::newly_held(&self.link.loops, &change, viewing);
        let first = self.first_round(&change);
        crate::core::apply_loops(&mut self.link.loops, change);
        if held && !first {
            let text = self.config.text.loops.held.clone();
            self.hint(text, false);
        }
    }

    /// 这一项是打了 `/loop` 新出现的那个循环交的第一次（核心的回执说了它的结果）：交过了就不再记着。
    fn first_round(&mut self, change: &LoopChange) -> bool {
        let LoopChange::One(id, Some(one)) = change else {
            return false;
        };
        let sent = &mut self.link.loops_sent;
        if sent.waiting && !self.link.loops.iter().any(|l| &l.id == id) {
            sent.fresh.insert(id.clone());
        }
        one.last.is_some() && sent.fresh.remove(id)
    }
}
