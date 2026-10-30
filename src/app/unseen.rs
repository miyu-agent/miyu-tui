//! 后台的会话做完了（蓝图 `tui.md`「后台命令、子代理和侧边栏」第 7 条「最近会话」，2026-10-30 项目主人：「会话在非聚焦
//! 情况下完成，没有任何提示」）：不是当前这个的会话一轮结束，侧边栏画圆点，弹「回答完了」，点了切过去；切过去就消。

use super::App;
use crate::core::SessionChange;

impl App {
    /// 会话列表推来的一项换上以前看：手上那一份在跑（`busy`）、推来的不在跑了，又不是当前这个的，记下、弹通知。
    pub(super) fn note_finished(&mut self, change: &SessionChange) {
        let SessionChange::Entry(info) = change else {
            return;
        };
        if info.busy || self.talk.transcript.session.as_deref() == Some(info.session.as_str()) {
            return;
        }
        let was_busy = self
            .talk
            .sessions_seen
            .iter()
            .flatten()
            .any(|s| s.session == info.session && s.busy);
        if !was_busy {
            return;
        }
        self.talk.unseen.insert(info.session.clone());
        let body = self.config.text.notify.replied.clone();
        let title = self.notice_title(&info.session);
        self.notifier
            .tell_notice(title, &body, Some(info.session.clone()));
    }
}
