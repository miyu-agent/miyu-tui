//! 会话读不了的两种（核心 YC-2、YC-6；蓝图 `tui.md`「连核心」第 7 条下）：
//!
//! - 日志中间坏了一行（`session_read_only`）：坏行以前的照最新一页画过了，订阅不上。切过去还在补发的先换上来，正文末尾红字
//!   写坏在第几行，输入框空着时写怎么新开会话；会话照旧列着、照旧对着它，说话核心照旧拒（弹「只能看」）。
//! - 一时读不了（`session_unavailable`）：载入、读日志时读写出错，日志没坏。弹一句「几秒后自动重试」，到点再读（读页没成的从读页起，
//!   订阅没成的只再订阅），最多试 [`TRIES`] 次，还不成的弹「请稍后重试」不再试；不说会话坏了，也不从列表里拿掉。

use std::time::{Duration, Instant};

use super::App;
use crate::core::Command;

/// 一时读不了以后隔多久再试（2026-11-10，YC-12：读写出错多半是打开文件数一下子用光这类，几秒就过去）。
pub const WAIT: Duration = Duration::from_secs(5);

/// 最多自动再试几次（一共等半分钟）。
pub const TRIES: u32 = 6;

/// 等着再试的那一个会话。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reread {
    /// 哪个会话。
    pub session: String,
    /// 从读最新一页起（不然只再订阅）。
    pub page: bool,
    /// 什么时候再试；试过了（`asked`）的是发出去的那一刻。
    pub due: Instant,
    /// 已经发出去再试了，等核心回。
    pub asked: bool,
    /// 已经试了几次。
    pub tries: u32,
}

impl App {
    /// 会话日志中间坏了一行：切过去还在补发的先换上来，正文末尾写坏在哪；停放着的照样记上（切回来看得到）。
    pub(super) fn session_read_only(&mut self, session: &str, line: Option<u64>) {
        if self.talk.opening(session) {
            self.opened();
        }
        let texts = &self.config.text.unread;
        let text = line.map_or_else(
            || texts.read_only_somewhere.clone(),
            |n| texts.read_only.replace("{line}", &n.to_string()),
        );
        if self.talk.transcript.session.as_deref() == Some(session) {
            self.talk.transcript.read_only(text);
        } else if let Some(parked) = self.talk.parked.get_mut(session) {
            parked.transcript.read_only(text);
        }
        if self
            .talk
            .reread
            .as_ref()
            .is_some_and(|r| r.session == session)
        {
            self.talk.reread = None;
        }
    }

    /// 会话一时读不了：记着到点再试、弹一句；同一个会话接着读不了的数着次数，到了 [`TRIES`] 次不再试。
    pub(super) fn session_unavailable(&mut self, session: String, page: bool) {
        let now = Instant::now();
        // 刚再试过、又读不了的接着数；隔了好一阵（上一回试成了以后又出的事）从头数
        let tries = match self.talk.reread.take() {
            Some(before)
                if before.session == session
                    && before.asked
                    && now.duration_since(before.due) < WAIT * 2 =>
            {
                before.tries
            }
            _ => 0,
        };
        let texts = &self.config.text;
        if tries >= TRIES {
            self.hint(texts.unread.unavailable_gave_up.clone(), false);
            return;
        }
        let seconds = WAIT.as_secs();
        let forms = &texts.retry_after.seconds;
        let form = if seconds == 1 { &forms[0] } else { &forms[1] };
        let wait = form.replace("{n}", &seconds.to_string());
        let text = texts.unread.unavailable_retry.replace("{wait}", &wait);
        self.talk.reread = Some(Reread {
            session,
            page,
            due: now + WAIT,
            asked: false,
            tries,
        });
        self.hint(text, false);
    }

    /// 到点了再试：还对着它（正在看、正在切过去）的照记着的那一种再读，切走了的不试了（`tick` 里叫）。
    pub(super) fn reread_due(&mut self, now: Instant) {
        let Some(due) = self
            .talk
            .reread
            .as_ref()
            .filter(|r| !r.asked && r.due <= now)
        else {
            return;
        };
        let session = due.session.clone();
        let still = self.talk.transcript.session.as_deref() == Some(session.as_str())
            || self.talk.opening(&session);
        let Some(mut next) = self.talk.reread.take().filter(|_| still) else {
            return;
        };
        next.tries += 1;
        // 再试成了照常画；又读不了的核心再回一次，[`App::session_unavailable`] 照发出去的时刻接着数
        next.asked = true;
        next.due = now;
        let (session, page) = (next.session.clone(), next.page);
        self.talk.reread = Some(next);
        self.link.core.send(Command::Reread { session, page });
    }
}
