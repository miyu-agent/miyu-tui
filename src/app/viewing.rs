//! 告诉核心这个头开着哪个会话（核心 HV-1，蓝图「后台命令、子代理和侧边栏」第 7 条子规则，2026-11-02 项目主人：herdr 里两个
//! 终端各弹一条「回答完了」，选「核心记谁开着哪个会话」）：开着的那个负责它的通知，别的头照会话列表的 `viewed` 不替它弹。

use super::App;
use crate::core::Command;

impl App {
    /// 每一帧：主会话报给 herdr（恢复命令）、告诉核心开着哪个；变了才说，连上、重连以后再说一遍（`updates.rs` 复位）。
    pub fn tell_main_session(&mut self) {
        let main = self.talk.main_session();
        self.notifier.session(main.as_deref());
        if self.link.viewing.as_ref() != Some(&main) {
            self.link.viewing = Some(main.clone());
            self.link.core.send(Command::Viewing(main));
        }
    }

    /// 别的头开着 `session`：会话列表里它那一项带 `viewed`，又不是这个头自己开着的。
    pub(super) fn viewed_elsewhere(&self, session: &str) -> bool {
        let mine = self
            .link
            .viewing
            .as_ref()
            .is_some_and(|v| v.as_deref() == Some(session));
        !mine
            && self
                .talk
                .sessions_seen
                .iter()
                .flatten()
                .any(|s| s.session == session && s.viewed)
    }
}
