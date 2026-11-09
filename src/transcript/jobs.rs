//! 后台任务结束的通知（蓝图 `tui.md`「后台命令、子代理和侧边栏」第 5 条）：正文末尾一行，点开看全文；后台命令的输出读回来
//! 以后照任务编号补进去。一轮进行中来的不切断时间线，当成在进行的那一段里的一步（发版前 check 21）。

use std::time::Duration;

use super::{JobMark, JobNote, Kind, ReportClass, Step, StepKind, Transcript};

impl Transcript {
    /// 正文末尾一条后台任务结束的通知（蓝图「后台命令、子代理和侧边栏」第 5 条）。
    pub fn job(&mut self, mark: JobMark, text: String, detail: String) {
        self.job_from(mark, text, detail, (None, ReportClass::Other));
    }

    /// 同上，一轮进行中也单独一行：抽屉里被拒的理由是人对那一步的回答，不是回报，收起来就看不到了（发版前 check 21）。
    pub fn job_line(&mut self, mark: JobMark, text: String, detail: String) {
        self.note(Kind::Job, text);
        if let Some(entry) = self.entries.last_mut() {
            entry.job = Some(JobNote {
                mark,
                detail,
                source: None,
            });
        }
    }

    /// 同上，记着是哪条任务的（任务编号）：后台命令的输出读回来了照它补进点开看的全文（[`Transcript::fill_job`]）；
    /// `class` 是收起那一行照它数的那一类。一轮进行中来的记进在进行的那一段（2026-11-09 项目主人从三个样板里挑了「放进
    /// 时间线里」：原来回报行把一段时间线切成两段，频繁时就是收起行和回报行交替）。
    pub fn job_from(
        &mut self,
        mark: JobMark,
        text: String,
        detail: String,
        (source, class): (Option<String>, ReportClass),
    ) {
        let note = JobNote {
            mark,
            detail,
            source,
        };
        if self.running.is_some() {
            let entry = self.open_segment();
            let mut step = Step::starting(StepKind::Report { note, text, class }, self.clock.now());
            step.took = Some(Duration::ZERO);
            if let Some(segment) = self.entries[entry].segment.as_mut() {
                segment.steps.push(step);
            }
            return;
        }
        self.note(Kind::Job, text);
        if let Some(entry) = self.entries.last_mut() {
            entry.job = Some(note);
        }
    }

    /// 任务 `source` 结束的那一行点开看的全文换成 `detail`（后台命令的输出读回来了）。
    pub fn fill_job(&mut self, source: &str, detail: String) {
        let note = self
            .entries
            .iter_mut()
            .rev()
            .flat_map(|e| {
                let steps = e.segment.iter_mut().flat_map(|s| s.steps.iter_mut().rev());
                let reported = steps.filter_map(|step| match &mut step.kind {
                    StepKind::Report { note, .. } => Some(note),
                    _ => None,
                });
                reported.chain(e.job.as_mut())
            })
            .find(|n| n.source.as_deref() == Some(source));
        if let Some(note) = note {
            note.detail = detail;
        }
    }
}
