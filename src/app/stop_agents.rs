//! 用 `Esc`、`Ctrl+C` 停子代理（蓝图 `tui.md`「按键」`Esc`、`Ctrl+C`，「切进子会话」第 4 条；2026-10-25 项目主人报：
//! 两下 `Esc` 打断了主会话，再按 `Esc`、`Ctrl+C` 都停不了子代理）。
//!
//! - 看着主会话、这一轮没在跑、还有子代理在跑：两下 `Esc`（或输入框空着时 `Ctrl+C`）停掉主会话派出去的全部子代理。
//!   主会话在回答时照旧只打断主会话（`13-终端界面.md` 第十节：打断只管正在看的会话）。
//! - 看着一个在跑的子代理：两下 `Esc`、`Ctrl+C` 停掉它（`job.stop`，父会话收到「被人停掉了」，不再白等），不只打断它
//!   这一轮；人留在它的画面里（项目主人定）。它停了、报完了，`Esc` 照旧一下回主会话。
//!
//! 停都对着主会话的任务表发：子代理是主会话派的，任务编号在主会话那边（同 `jobs.rs` 的 `x`）。

use std::time::{Duration, Instant};

use super::App;
use crate::core::Command;

impl App {
    /// 这时按 `Esc`、`Ctrl+C` 要停的子代理（主会话的任务编号）：看着子代理的是它自己（在跑的话），看着闲着的主会话是
    /// 它派出去、在跑的全部。主会话在回答、切会话还没换上来的时候是空的。
    pub(super) fn agents_to_stop(&self) -> Vec<String> {
        if self.talk.opening.is_some() {
            return Vec::new();
        }
        let tree = self.talk.tree();
        let running = tree.agents();
        match self.talk.viewing() {
            Some(child) => running
                .into_iter()
                .filter(|j| j.session.as_deref() == Some(child))
                .map(|j| j.job.clone())
                .collect(),
            None if self.talk.transcript.running.is_none()
                && self.talk.transcript.starting == crate::transcript::Starting::No =>
            {
                running.into_iter().map(|j| j.job.clone()).collect()
            }
            None => Vec::new(),
        }
    }

    /// 停掉 [`App::agents_to_stop`] 那几个：对着主会话发 `job.stop`，回报来了照常出「已停止」。
    pub(super) fn stop_agents(&mut self) {
        let session = self.talk.main_session();
        for job in self.agents_to_stop() {
            self.link.core.send(Command::Stop {
                session: session.clone(),
                job,
            });
        }
    }

    /// 两下 `Esc` 停子代理：上一下还在时限里就停，不然只提示再按一次。
    pub(super) fn esc_agents(&mut self) {
        let window = Duration::from_millis(self.config.layout.esc_window_ms);
        match self.typing.esc_at.take() {
            Some(at) if at.elapsed() <= window => {
                self.typing.notice = None;
                self.stop_agents();
            }
            _ => {
                self.typing.esc_at = Some(Instant::now());
                self.hint(self.config.text.esc_agents_hint.clone(), false);
            }
        }
    }
}
