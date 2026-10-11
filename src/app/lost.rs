//! 断线时在途的请求丢了（review B5，2026-10-11）：核心不会再回，等着回应的几处复位，该再要的再要。再要的命令在通道里
//! 排着，重连上再发（「连核心」第 1、7 条）。原来只有 mermaid 图忘掉没回来的，任务的输出、更早的一页、链接卡片一直等着。

use super::App;

impl App {
    /// 核心断开了：任务的输出、更早的一页、链接卡片和 mermaid 图，要了还没回来的都复位。
    pub(super) fn lost_requests(&mut self) {
        // 任务的输出：在跑的照常隔一会儿再读；结束了的重读全部的（正文里那一行点开看）。
        let mut finals = Vec::new();
        let boards = std::iter::once((self.transcript.session.clone(), &mut self.board)).chain(
            self.parked
                .iter_mut()
                .map(|(session, p)| (Some(session.clone()), &mut p.board)),
        );
        for (session, board) in boards {
            for job in board.jobs.iter_mut().filter(|j| j.waiting) {
                job.waiting = false;
                job.asked = None;
                if let Some(session) = session
                    .as_ref()
                    .filter(|_| !job.running() && job.output.is_none())
                {
                    finals.push((session.clone(), job.job.clone()));
                }
            }
        }
        for (session, job) in finals {
            self.fetch_final(&session, &job);
        }
        // 更早的一页：那一行露着时再要。
        self.transcript.lost();
        for parked in self.parked.values_mut() {
            parked.transcript.lost();
        }
        // 卡片、图：没回来的忘掉，排好的行扔掉，重排时再要（排好的行里记着占位，不重排就不会再问）。
        self.cards.borrow_mut().lost();
        self.diagrams.borrow_mut().lost();
        *self.row_cache.borrow_mut() = Default::default();
    }
}
