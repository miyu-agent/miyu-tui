//! 给人的通知（核心 J-3 中，蓝图 `tui.md`「系统通知」第 8 条；2026-10-27 项目主人照推荐定：照 Claude Code，人不在才弹）：
//! 每次新开会话的定时跑完了，标题写那个会话（标题，没有的照预览：就是那句话的开头），正文写跑完了没有，点开切过去；唤醒表
//! 坏了换了新的，说一声原来的定时都没了。弹不弹照「系统通知」的规矩（终端在前台时 kitty 自己不弹，别的终端只在报过离开
//! 时弹）。

use super::App;
use crate::core::Notice;

/// 标题最多几个字：通知上放不下的系统会截，先截短免得截在半个字上。
const TITLE_CHARS: usize = 40;

impl App {
    /// 核心推来一条给人的通知。
    pub(super) fn notice(&mut self, notice: &Notice) {
        let texts = &self.config.text.notify;
        match notice {
            Notice::ScheduleRan { session, ok } => {
                let body = if *ok {
                    texts.schedule_ran.clone()
                } else {
                    texts.schedule_failed.clone()
                };
                let title = self.notice_title(session);
                self.notifier
                    .tell_notice(title, &body, Some(session.clone()));
            }
            Notice::SchedulesReset => {
                let body = texts.schedules_reset.clone();
                self.notifier.tell_notice(None, &body, None);
            }
        }
    }

    /// 终端往输入里回的一句话（`reader/replies.rs`）：点了的 kitty 通知切过去，别的不管。
    pub fn terminal_reply(&mut self, reply: &str) {
        if let Some(session) = crate::notify::clicked(reply) {
            self.open_clicked(session);
        }
    }

    /// 被叫醒时看一眼：系统通知点了的（`notify-send` 在另一个线程里等着）切过去。
    pub(super) fn clicked_notices(&mut self) {
        if let Some(session) = self.notifier.take_clicked().pop() {
            self.open_clicked(session);
        }
    }

    /// 切到点了的那个会话。配置页开着的不关（窗里可能正改着东西），在底下切，关了配置页就是它。
    fn open_clicked(&mut self, session: String) {
        self.switch_session(session);
    }

    /// 那个会话在会话列表里怎么叫：标题，没有的照预览；没见过的是 `None`（写 Miyu）。
    fn notice_title(&self, session: &str) -> Option<String> {
        let info = self
            .talk
            .sessions_seen
            .iter()
            .flatten()
            .find(|s| s.session == session)?;
        let name = info.title.as_deref().or(info.preview.as_deref())?;
        Some(name.chars().take(TITLE_CHARS).collect())
    }
}
