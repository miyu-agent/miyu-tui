//! 换会话时换输入框（发版前 check 第 17 条，2026-11-09 项目主人：框里的字切会话时跟了过去；`Ctrl+C` 清掉、按 `↑` 找回的
//! 那一份也跟着会话走）：现在这个会话的一份（`crate::input::Desk`）记在它名下，换上来的放回它自己的。还没开会话的
//! 新会话页上的不记（蓝图「后台命令、子代理和侧边栏」第 7 条最近会话的子条）。

use super::App;

impl App {
    /// 换到 `to`（`None` 是 `/new` 的新会话页）：输入框里现在这一份记在正在看的会话名下（空的不记），换上 `to` 自己的。
    /// `@` 文件列表从头认词；`/jump` 列的是原来那个会话说的话，开着的关掉。
    pub(super) fn swap_desk(&mut self, to: Option<&str>) {
        let desk = self.typing.input.take_desk();
        if let Some(was) = self.talk.transcript.session.clone() {
            if desk.is_empty() {
                self.typing.drafts.remove(&was);
            } else {
                self.typing.drafts.insert(was, desk);
            }
        }
        let own = to.and_then(|to| self.typing.drafts.remove(to));
        self.typing.input.put_desk(own.unwrap_or_default());
        self.typing.mention.forget();
        if self.typing.history.jump {
            self.typing.history.close();
        }
    }
}
