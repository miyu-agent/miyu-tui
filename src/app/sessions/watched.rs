//! 监视的命令出的行落进正文（蓝图 `tui.md`「后台命令、子代理和侧边栏」第 5 条子条，排期第 37 条，核心 MN-1）。

use crate::config::JobTexts;
use crate::core::JobLines;
use crate::jobs::Board;
use crate::transcript::{JobMark, Transcript};

/// 监视的命令出了一批行：正文末尾一行「监视 · 标题 · N 行」，记号强调色、字暗；点开看这一批（排期第 37 条，核心设计 56
/// 「照回报那样画一行、点开看全文」）。不记任务编号：命令了结时读回来的整份输出补进了结那一行，不补进这里。
pub(super) fn lines(
    board: &Board,
    transcript: &mut Transcript,
    lines: &JobLines,
    words: &JobTexts,
) {
    let title = board
        .by_job(&lines.job)
        .map_or_else(|| lines.job.clone(), |job| job.headline());
    let text = words
        .lines_note
        .replace("{title}", &title)
        .replace("{lines}", &lines.lines.to_string());
    transcript.job(JobMark::Lines, text, lines.text.clone());
}
