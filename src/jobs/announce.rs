//! 后台任务结束的那一行（蓝图 `tui.md`「后台命令、子代理和侧边栏」第 5 条）：全屏、shellhook 同一份（从 `app/sessions/notes.rs`
//! 挪来）。

use std::time::Instant;

use crate::config::JobTexts;
use crate::jobs::{Board, JobKind, JobState};
use crate::meter;
use crate::transcript::{JobMark, Transcript};

/// 一条后台任务结束了：正文末尾一行，记号有颜色、字暗；点开看全文（「后台命令、子代理和侧边栏」第 5 条）。
pub fn announce(
    board: &Board,
    transcript: &mut Transcript,
    id: u64,
    now: Instant,
    words: &JobTexts,
) {
    let Some(job) = board.jobs.iter().find(|j| j.id == id) else {
        return;
    };
    let elapsed = meter::clock(job.elapsed(now).as_secs());
    let why = match job.state {
        JobState::Undone => words.why_undone.as_str(),
        JobState::Restarted => words.why_restarted.as_str(),
        JobState::TimedOut => words.why_timed_out.as_str(),
        JobState::TooLarge => words.why_too_large.as_str(),
        _ => "",
    };
    let stopped = matches!(
        job.state,
        JobState::Stopped
            | JobState::Undone
            | JobState::Restarted
            | JobState::TimedOut
            | JobState::TooLarge
    );
    let (mark, text) = match (job.kind, job.state) {
        (JobKind::Agent, _) if stopped => (JobMark::Stopped, words.agent_stopped_note.clone()),
        (JobKind::Agent, _) => (JobMark::Done, words.agent_note.clone()),
        (_, JobState::Failed(code)) => (
            JobMark::Failed,
            words.failed_note.replace("{code}", &code.to_string()),
        ),
        (_, JobState::Killed(signal)) => (
            JobMark::Failed,
            words.killed_note.replace("{signal}", &signal.to_string()),
        ),
        _ if stopped => (JobMark::Stopped, words.stopped_note.clone()),
        _ => (JobMark::Done, words.done_note.clone()),
    };
    let text = text
        .replace("{title}", &job.headline())
        .replace("{elapsed}", &elapsed)
        .replace("{why}", why);
    // 点开看的全文：子代理是交回来的回答；命令先是命令本身，输出读回来了接在后面（`app/output.rs`）。
    let detail = match job.kind {
        JobKind::Agent => job.report.clone(),
        JobKind::Shell => job.title.trim().to_string(),
    };
    transcript.job_from(mark, text, detail, Some(job.job.clone()));
}
