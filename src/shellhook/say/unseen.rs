//! 她还没看到的回报（`say` 和盯后台的共用）：后台任务结束落了盘（`job.reported`、`child.reported`），她这一轮之后的请求
//! 没带上的，核心在这一轮结束时、闲着的当场叫醒她开一轮，`trigger` 是其中最后一条的序号（核心 `session/jobs.rs`）。照序号
//! 认叫醒的是哪一轮，不靠「下一个不是人说话开的」去猜（2026-10-20 验收：猜错了，人说话开的那一轮被当成叫醒的又印一遍）。

use crate::core::JobReason;

/// 还没看到的回报：序号、是不是交给这里盯的任务。
#[derive(Debug, Default)]
pub struct Unseen {
    reports: Vec<(u64, bool)>,
    /// 刚推来的 `Reported`：配给紧跟着的那条 `JobEnded`。
    next: Option<u64>,
}

impl Unseen {
    /// 下一条任务结束落盘的序号。
    pub fn reported(&mut self, seq: u64) {
        self.next = Some(seq);
    }

    /// 那个任务结束了：`ours` 是不是这里盯的。撤销、重启停掉的不叫醒她，不记。
    pub fn ended(&mut self, ours: bool, reason: JobReason) {
        let wakes = matches!(reason, JobReason::Finished | JobReason::Stopped);
        if let Some(seq) = self.next.take()
            && wakes
        {
            self.reports.push((seq, ours));
        }
    }

    /// 一次请求看到了第 `seen` 条为止：带上了的不会再叫醒她（她自己停的那种也在这里消掉）。
    pub fn heard(&mut self, seen: u64) {
        self.reports.retain(|(seq, _)| *seq > seen);
    }

    /// 开了一轮：`trigger` 是还没看到的回报的，就是叫醒的那一轮，交回里面有没有这里盯的；别的（人说话开的）交回 `None`。
    pub fn woke(&mut self, trigger: Option<u64>) -> Option<bool> {
        let trigger = trigger?;
        if !self.reports.iter().any(|(seq, _)| *seq == trigger) {
            return None;
        }
        let ours = self
            .reports
            .iter()
            .any(|(seq, ours)| *ours && *seq <= trigger);
        self.reports.retain(|(seq, _)| *seq > trigger);
        Some(ours)
    }

    /// 还有这里盯的没看到：要等叫醒她的那一轮。
    pub fn owed(&self) -> bool {
        self.reports.iter().any(|(_, ours)| *ours)
    }
}

#[cfg(test)]
mod tests {
    use super::Unseen;
    use crate::core::JobReason;

    #[test]
    fn the_woken_turn_is_known_by_its_trigger() {
        let mut unseen = Unseen::default();
        unseen.reported(379);
        unseen.ended(true, JobReason::Finished);
        assert!(unseen.owed());
        assert_eq!(unseen.woke(Some(390)), None, "人说话开的");
        assert_eq!(unseen.woke(Some(379)), Some(true));
        assert!(!unseen.owed());
    }

    #[test]
    fn a_report_she_already_saw_wakes_nothing() {
        // 前台 sleep 的时候后台那个结束了，下一次请求带上了：不叫醒（2026-10-20 验收 SH-30）。
        let mut unseen = Unseen::default();
        unseen.reported(396);
        unseen.ended(true, JobReason::Finished);
        unseen.heard(398);
        assert!(!unseen.owed());
    }

    #[test]
    fn undone_and_restarted_jobs_do_not_wake_her() {
        let mut unseen = Unseen::default();
        unseen.reported(5);
        unseen.ended(true, JobReason::Undone);
        unseen.reported(6);
        unseen.ended(true, JobReason::Restarted);
        assert!(!unseen.owed());
    }
}
