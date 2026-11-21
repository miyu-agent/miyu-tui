//! 唤醒表里的循环变了（蓝图 `tui.md`「循环」第 1 条）：改手上那一份；看着的会话上的循环这一次只记下、没开轮（`last.result`
//! 是 `held`），底下提示一句（ACC-1121-#1，2026-11-20 主人本机：撤销了一轮、还能恢复时打的 `/loop` 一直不跑，界面上什么都
//! 没有）。`/loop` 收下以后核心马上交第一次，这一句常比命令的回执先到：两句都写，回执在上，不互相顶掉。

use std::time::Instant;

use super::App;
use crate::core::LoopChange;

impl App {
    /// 唤醒表推来一项变化。
    pub(super) fn loops_changed(&mut self, change: LoopChange) {
        let viewing = self.talk.transcript.session.as_deref();
        let held = crate::core::newly_held(&self.link.loops, &change, viewing);
        crate::core::apply_loops(&mut self.link.loops, change);
        if !held {
            return;
        }
        let said = self.config.text.loops.held.clone();
        let text = match self.showing() {
            Some(receipt) if receipt != said => format!("{receipt}\n{said}"),
            _ => said,
        };
        self.hint(text, false);
    }

    /// 命令的回执：底下还写着「循环这一轮没开始」的，回执写在它上面一行。
    pub(super) fn receipt(&mut self, text: String, tail: String) {
        let said = &self.config.text.loops.held;
        let text = match self.showing() {
            Some(shown) if shown.ends_with(said.as_str()) => format!("{text}\n{said}"),
            _ => text,
        };
        self.hint_keeping(text, tail, false);
    }

    /// 底下正写着、没到点收的那一句（不带接在后面整个留着的那一截的）。
    fn showing(&self) -> Option<String> {
        let notice = self.typing.notice.as_ref()?;
        (notice.until > Instant::now() && notice.tail.is_empty()).then(|| notice.text.clone())
    }
}
