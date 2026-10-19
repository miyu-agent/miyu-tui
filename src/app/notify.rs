//! 界面这一边什么时候通知（蓝图 `tui.md`「系统通知」第 1、6 条）：一轮结束弹「回答好了」「出错了」，抽屉打开弹
//! 「在等你确认」「在等你回答」；在 herdr 里报在做、在等、空闲。弹不弹、响不响由 `notify::plan` 定。长期目标还在跑的，
//! 「回答好了」先欠着，目标停下来才弹（2026-10-17）。

use super::App;
use crate::core::{EndReason, GoalState, Push, Update};
use crate::notify::{Event, State};

impl App {
    /// 核心的一条消息过一遍通知，在正文收它之前：没有在跑的一轮的结束不弹（核心重启以后补推的），手动压缩、清空那一轮
    /// 不弹。
    pub(super) fn notify_core(&mut self, update: &Update) {
        // 补发来的是以前的事：不弹、不响、不报；补完了照这时在不在跑报一次 herdr（「会话列表」第 6 条）。
        if self.talk.transcript.replaying() {
            if let Update::Push(Push::Clock(None)) = update {
                let running = self.talk.transcript.running.is_some();
                self.notifier
                    .state(herdr_state(self.talk.drawers.open(), running));
            }
            return;
        }
        let seen = self.talk.transcript.running.is_some();
        let manual = self.talk.transcript.manual_turn();
        self.introduce();
        match update {
            // 一轮开始：有抽屉开着的照旧在等（别处来的话可以在抽屉开着时开一轮）。
            Update::Push(Push::TurnStarted(..)) => {
                // 接着开了一轮（续轮、人说的话）：欠着的「回答好了」不弹了。
                self.notifier.forgive();
                self.notifier
                    .state(herdr_state(self.talk.drawers.open(), true));
            }
            // 长期目标停下来了（等人开口、停着、卡住、做完、清掉）：欠着的现在弹。
            Update::Push(Push::Goal(goal)) => {
                if !goal.as_ref().is_some_and(running) {
                    self.notifier.settle();
                }
            }
            Update::Push(Push::TurnEnded(reason)) => {
                if seen && !manual {
                    match reason {
                        // 长期目标还在跑：后面多半接着开续轮，先欠着。
                        EndReason::Completed
                            if self.talk.transcript.goal.as_ref().is_some_and(running) =>
                        {
                            self.notifier.owe(Event::Replied);
                        }
                        EndReason::Completed => self.notifier.tell(Event::Replied),
                        EndReason::Error => self.notifier.tell(Event::Failed),
                        EndReason::Interrupted | EndReason::Other(_) => {}
                    }
                }
                // 一轮结束：还有开着的抽屉的在等，不然空闲。
                self.notifier
                    .state(herdr_state(self.talk.drawers.open(), false));
            }
            Update::Disconnected => {
                self.notifier.forgive();
                self.notifier.state(State::Idle);
            }
            _ => {}
        }
    }

    /// 抽屉开了一个（新开的、了结一个轮到下一个的）：弹「在等你确认」「在等你回答」，报在等。
    pub(super) fn notify_drawer(&mut self) {
        if self.talk.drawers.current.is_none() {
            return;
        }
        self.introduce();
        let Some(drawer) = self.talk.drawers.current.as_ref() else {
            return;
        };
        let event = if drawer.is_question() {
            Event::Asking
        } else {
            Event::Approving
        };
        self.notifier.tell(event);
        self.notifier.state(State::Blocked);
    }

    /// 没有在等你的了：一轮还在跑的在做，不然空闲；还有开着的抽屉的在等。
    pub(super) fn settle_state(&mut self) {
        let state = herdr_state(
            self.talk.drawers.open(),
            self.talk.transcript.running.is_some(),
        );
        self.notifier.state(state);
    }
}

impl App {
    /// 接下来的通知是这个会话的（「系统通知」第 3 条，2026-10-17 项目主人定）：标题是会话标题，图标是这个会话的人格头像
    /// （存下来了的；终端显示不了图也照样用）。
    fn introduce(&mut self) {
        let title = self.talk.transcript.title.clone();
        let avatar = self
            .seen()
            .wanted(super::avatars::Place::Sidebar)
            .and_then(|(_, version)| self.media.avatars.file(version));
        self.notifier.about(title, avatar);
        self.notify_mascot();
    }

    /// 没有头像时通知的图标照现在的吉祥物画（「系统通知」第 3 条，2026-10-19）：颜色固定照网页晨光那一套。
    pub(super) fn notify_mascot(&mut self) {
        self.notifier.mascot(self.config.mascot.clone());
    }
}

/// 长期目标在跑：一轮结束以后核心会接着开续轮。
fn running(goal: &crate::core::Goal) -> bool {
    goal.state == GoalState::Running
}

/// 在 herdr 里报哪种（蓝图「系统通知」第 6 条）：有抽屉开着就是在等，在等你比在做要紧；不然一轮在跑是在做，都没有是空闲。
fn herdr_state(drawer_open: bool, running: bool) -> State {
    if drawer_open {
        State::Blocked
    } else if running {
        State::Working
    } else {
        State::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::herdr_state;
    use crate::notify::State;

    #[test]
    fn an_open_drawer_outranks_a_running_turn() {
        // 2026-10-01 查出来：别处来的话在抽屉开着时开了一轮，原来报成在做，一轮结束才改回在等。
        assert_eq!(herdr_state(true, true), State::Blocked);
        assert_eq!(herdr_state(true, false), State::Blocked);
        assert_eq!(herdr_state(false, true), State::Working);
        assert_eq!(herdr_state(false, false), State::Idle);
    }
}
