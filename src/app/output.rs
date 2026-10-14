//! 后台命令的输出（蓝图 `tui.md`「后台命令、子代理和侧边栏」第 3、5 条，`protocol.md` 的 `job.output`）：面板里点开的
//! 那条在跑的，头一次要最后几行，之后照 `size` 带 `after`、`wait` 接着读，长了核心才答，不定时去问（核心 V-6 上，设计 00
//! 「避免轮询」）；旧核心不交 `size` 的照旧隔 `poll_ms` 要一次。一条命令结束了读它全部的（最多 `note_lines` 行），
//! 正文里它那一行点开看。

use std::time::{Duration, Instant};

use super::{App, Panel};
use crate::core::{Command, JobOutput, OutputRead};
use crate::jobs::clean::note_detail;

/// 面板里点开的那一条下一次怎么读；不用读的是 `None`。`asked` 是上一次要最后几行的时刻（旧核心照它隔 `every` 再要）。
pub fn next_read(
    running: bool,
    output: Option<&JobOutput>,
    (asked, now, every): (Option<Instant>, Instant, Duration),
    (rows, lines): (usize, usize),
) -> Option<OutputRead> {
    if !running {
        return output.is_none().then_some(OutputRead::Tail(lines));
    }
    match output.and_then(|o| o.size.map(|size| (size, o.more))) {
        // 接着读：上一段后面还有的当场接着要，不然等长了再答。
        Some((size, more)) => Some(OutputRead::After { size, wait: !more }),
        // 头一次；旧核心（不交 `size`）照旧隔 `every` 要一次最后几行。
        None => asked
            .is_none_or(|at| now >= at + every)
            .then_some(OutputRead::Tail(rows)),
    }
}

impl App {
    /// 面板里点开的那一条：照 [`next_read`] 读。上一次还没回来的不再发（等着的那一条长了才回来）。
    pub(super) fn poll_output(&mut self) {
        let Some(Panel::Background { open: Some(id), .. }) = self.boxes.panel else {
            return;
        };
        let Some(session) = self.talk.transcript.session.clone() else {
            return;
        };
        let look = &self.config.layout.output;
        let (rows, lines) = (look.panel_rows, look.note_lines);
        let every = Duration::from_millis(look.poll_ms);
        let now = Instant::now();
        let Some(job) = self.talk.board.jobs.iter_mut().find(|j| j.id == id) else {
            return;
        };
        if job.waiting {
            return;
        }
        let Some(read) = next_read(
            job.running(),
            job.output.as_ref(),
            (job.asked, now, every),
            (rows, lines),
        ) else {
            return;
        };
        job.waiting = true;
        if matches!(read, OutputRead::Tail(_)) {
            job.asked = Some(now);
        }
        let job = job.job.clone();
        self.link.core.send(Command::Output { session, job, read });
    }

    /// 会话 `session` 里的命令 `job` 结束了：读它全部的输出，正文里它那一行点开看（第 5 条）。
    pub(super) fn fetch_final(&mut self, session: &str, job: &str) {
        let tail = self.config.layout.output.note_lines;
        if let Some((_, board)) = self.slot(session)
            && let Some(entry) = board.by_job_mut(job)
        {
            entry.waiting = true;
            entry.asked = Some(Instant::now());
            let (session, job) = (session.to_string(), job.to_string());
            let read = OutputRead::Tail(tail);
            self.link.core.send(Command::Output { session, job, read });
        }
    }

    /// 核心交回了一条命令的输出：记进那个会话的任务表；结束了的补进正文里它那一行（洗过、去掉末尾的空行；前面还有
    /// 没交的，第一行写 `…`；一个字都没有的写「没有输出」）。
    pub(super) fn took_output(
        &mut self,
        session: &str,
        job: &str,
        output: Option<JobOutput>,
        after: bool,
    ) {
        let tab = self.config.layout.output.tab;
        let keep = self.config.layout.output.panel_rows;
        let nothing = self.config.text.jobs.no_output.clone();
        let Some((transcript, board)) = self.slot(session) else {
            return;
        };
        let Some(entry) = board.by_job_mut(job) else {
            return;
        };
        entry.waiting = false;
        let Some(output) = output else {
            return;
        };
        // 接着读的一段接在已有的后面（面板只露最后几行，留那么多）。
        let output = match (after, entry.output.take()) {
            (true, Some(mut had)) => {
                had.append(&output, keep);
                had
            }
            _ => output,
        };
        // 正文里那一行点开：命令、空行、输出（照时间线点开一条命令，2026-09-30 项目主人：原来只看得到输出）。接着读的那
        // 一段只留着面板的几行，全部的等结束时整份读的那一次（`fetch_final`）。
        if !output.running && !after {
            let detail = note_detail(&output, tab, &nothing);
            transcript.fill_job(job, format!("{}\n\n{detail}", entry.title.trim()));
        }
        entry.output = Some(output);
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::next_read;
    use crate::core::{JobOutput, OutputRead};

    #[test]
    fn a_running_command_is_followed_by_waiting_reads_and_an_old_core_by_the_clock() {
        let now = Instant::now();
        let every = Duration::from_secs(1);
        let read = |running, output: Option<&JobOutput>, asked| {
            next_read(running, output, (asked, now, every), (12, 2000))
        };
        // 头一次要最后几行。
        assert_eq!(read(true, None, None), Some(OutputRead::Tail(12)));
        // 交了 `size` 的：接着读、等长了再答；这一段后面还有的当场接着要。
        let mut had = JobOutput {
            size: Some(40),
            running: true,
            ..JobOutput::default()
        };
        assert_eq!(
            read(true, Some(&had), Some(now)),
            Some(OutputRead::After {
                size: 40,
                wait: true
            })
        );
        had.more = true;
        assert_eq!(
            read(true, Some(&had), Some(now)),
            Some(OutputRead::After {
                size: 40,
                wait: false
            })
        );
        // 旧核心不交 `size`：照旧隔一秒要一次最后几行。
        let old = JobOutput::default();
        assert_eq!(read(true, Some(&old), Some(now)), None);
        assert_eq!(
            read(true, Some(&old), Some(now - every)),
            Some(OutputRead::Tail(12))
        );
        // 结束了：没读过的整份读一次，读过的不再读。
        assert_eq!(read(false, None, None), Some(OutputRead::Tail(2000)));
        assert_eq!(read(false, Some(&had), None), None);
    }
}
