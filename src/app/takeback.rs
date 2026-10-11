//! Ctrl+C 打断时退回排着的话（蓝图 `tui.md`「按键」`Ctrl+C`，2026-09-30 项目主人定只退回排着的）：还排着的核心照常退回；
//! 这一轮是排着的话开的、她还没开口的，那几句已经发给模型、核心不退，打断以后等这一轮结束接着撤掉它，照排着时的样子
//! 放回输入框。

use super::{App, paste};
use crate::core::{Command, Push, Update};
use crate::input::Draft;
use crate::transcript::Chip;

/// 退回的那几句：话和它的粘贴块。
type Back = Vec<(String, Vec<Chip>)>;

/// 退回走到哪一步。都记着是哪个会话打断的：正在看的换了（`/new`、切走了）、断开了，那一轮的结束不会再从这里来，
/// 这件事作废，不拿别的回合当它（review A2，2026-10-11：原来留着，下一轮正常结束就被撤掉，连改过的文件一起还原）。
#[derive(Debug, PartialEq)]
pub enum Takeback {
    /// 打断发出去了，等这一轮结束：哪个会话、那几句。
    Interrupting(Option<String>, Back),
    /// 撤销发出去了，等撤销成了：哪个会话、那几句。
    Reverting(Option<String>, Back),
}

impl Takeback {
    /// 哪个会话打断的。
    fn session(&self) -> Option<&str> {
        match self {
            Takeback::Interrupting(session, _) | Takeback::Reverting(session, _) => {
                session.as_deref()
            }
        }
    }
}

/// 收到一条消息以后做什么。
#[derive(Debug, PartialEq)]
enum Step {
    /// 接着等。
    Wait(Takeback),
    /// 这一轮结束了：发撤销，等它成。
    Revert(Takeback),
    /// 撤销成了：放回输入框。
    Return(Back),
    /// 作废。
    Drop,
}

/// 照消息定下一步；`viewing` 是正在看的会话。
fn step(taken: Takeback, update: &Update, viewing: Option<&str>) -> Step {
    if taken.session() != viewing || matches!(update, Update::Disconnected) {
        return Step::Drop;
    }
    match (taken, update) {
        (Takeback::Interrupting(session, back), Update::Push(Push::TurnEnded(_))) => {
            Step::Revert(Takeback::Reverting(session, back))
        }
        (Takeback::Reverting(_, back), Update::Undone { restore: false, .. }) => Step::Return(back),
        (Takeback::Reverting(..), Update::Refused { .. }) => Step::Drop,
        (kept, _) => Step::Wait(kept),
    }
}

impl App {
    /// Ctrl+C 打断（排着的退回）：这一轮是排着的话开的、她还没开口的，记下那几句，等结束了撤掉这一轮。
    pub(super) fn interrupt(&mut self) {
        if self.switching() {
            return;
        }
        let session = self.transcript.session.clone();
        self.takeback = self
            .transcript
            .takeback()
            .map(|back| Takeback::Interrupting(session, back));
        self.core.send(Command::Interrupt { send: false });
    }

    /// 正在看的会话推来的：这一轮结束了发撤销；撤销成了把那几句放回输入框；撤销被拒了、看的换了、断开了就算了。
    pub(super) fn takeback_on(&mut self, update: &Update) {
        let Some(taken) = self.takeback.take() else {
            return;
        };
        match step(taken, update, self.transcript.session.as_deref()) {
            Step::Wait(kept) => self.takeback = Some(kept),
            Step::Revert(next) => {
                self.core.send(Command::Revert);
                self.takeback = Some(next);
            }
            Step::Return(back) => self.put_returned(back),
            Step::Drop => {}
        }
    }

    /// 退回的话连同粘贴块放回输入框，一条之间空一行，接在已有的字前面（`tui.md`「输入框」第 8、11 条）。输入历史里
    /// 找得到的照发出去时的样子：附件跟着回来。
    pub(super) fn put_returned(&mut self, returned: Back) {
        if returned.is_empty() {
            return;
        }
        let mut draft = Draft::default();
        for (text, chips) in returned {
            let sent = self.input.sent_by_text(&text);
            let back = sent.unwrap_or_else(|| Draft::from_pasted(&text, &paste::pasted(chips)));
            draft.append(back, "\n\n");
        }
        if !self.input.editor.is_empty() {
            draft.append(self.input.draft(), "\n\n");
        }
        self.input.editor.set_draft(draft);
    }
}

#[cfg(test)]
mod tests {
    use super::{Step, Takeback, step};
    use crate::core::{EndReason, Push, Update};

    fn waiting(session: &str) -> Takeback {
        Takeback::Interrupting(
            Some(session.into()),
            vec![("排着的那句".into(), Vec::new())],
        )
    }

    fn ended() -> Update {
        Update::Push(Push::TurnEnded(EndReason::Interrupted))
    }

    #[test]
    fn only_the_interrupted_session_ending_its_turn_is_reverted() {
        // review A2（2026-10-11）：打断以后马上 /new、切走，那一轮的结束推到了停放的会话；正在看的这一个下一轮结束时
        // 不能当成它撤掉。
        assert!(matches!(
            step(waiting("a"), &ended(), Some("a")),
            Step::Revert(_)
        ));
        assert_eq!(
            step(waiting("a"), &ended(), Some("b")),
            Step::Drop,
            "看的换了"
        );
        assert_eq!(
            step(waiting("a"), &ended(), None),
            Step::Drop,
            "/new 以后还没开会话"
        );
        assert_eq!(
            step(waiting("a"), &Update::Disconnected, Some("a")),
            Step::Drop,
            "断开了：界面自己收尾，不会再有这一轮的结束"
        );
    }
}
