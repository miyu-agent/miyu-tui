//! `shellhook say`：把一句话交给她，这一轮照终端界面时间线的样子印在 shell 里（图纸 `shellhook.md`「say」）。
//!
//! 连核心、收推送、排时间线都照全屏那一套（`core::spawn`、`Transcript`、`ui/rows.rs`），只是不进全屏：光标下面一块活动区
//! 原地刷新（`paint.rs`、`session.rs`），这一轮结束时整段定下来留在滚动区。说到哪个会话问核心（`session.terminal`，
//! `terminal.rs`）。答完还有派出去没结束的后台任务的，起一个盯后台的（`follow.rs`），回报回来时 shell 在提示符上印
//! （`report.rs`）。

mod asking;
pub mod follow;
mod look;
mod paint;
pub mod report;
mod session;
pub mod spool;
mod terminal;
mod turn;
mod unseen;
mod view;

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::shellhook::words::Words;
use crate::transcript::Link;
use session::Session;
use turn::Turn;

/// 按了 Ctrl+C 打断的退出码：同 `miyu ask`（蓝图 `cli/ask.md`）。
const INTERRUPTED: u8 = 3;

/// 照这几样办。
pub struct Ask {
    /// 说的那句话。
    pub text: String,
    /// 这期间提前敲的字写进这个文件，脚本读了放回命令行；没给的不收。
    pub leftover: Option<PathBuf>,
    /// 哪个 shell（脚本给的 `$fish_pid`）：后台回报交给它。没给的不盯后台。
    pub shell: Option<u32>,
}

/// 说一句、印这一轮，交回退出码。
///
/// # Errors
///
/// 读配置、开原始模式、写终端失败。
pub fn run(ask: Ask, words: &Words) -> io::Result<ExitCode> {
    let text = ask.text.trim_end_matches(['\n', '\r']).to_string();
    if text.trim().is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    // 终端界面的斜杠命令（`/models`）：照命令办，不当一句话（`shellhook/slash.rs`）。
    if text.starts_with('/')
        && let Some(done) = crate::shellhook::slash::run(&text, words)
    {
        return done;
    }
    // 这一句里 `Ctrl+V` 贴的块：换回原文、带上附件（`shellhook/paste.rs`）。
    let (text, files) = match ask.shell {
        Some(shell) => crate::shellhook::paste::take(shell, &text),
        None => (text, Vec::new()),
    };
    let (mut session, look) = Session::start()?;
    // 这一句照 shell 所在的目录说（设计 22 第七节「她在哪个目录干活」）。
    let cwd = std::env::current_dir()
        .ok()
        .map(|d| d.to_string_lossy().into_owned());
    let mut turn = Turn::new(text, cwd, look);
    turn.files = files;
    turn.ask_terminal(&session.core);
    session.drive(&mut turn, |t| {
        t.renewed.map(|since| renewed_line(since, words))
    })?;
    session.finish()?;
    if let Some(path) = &ask.leftover
        && !turn.typed.is_empty()
    {
        std::fs::write(path, &turn.typed)?;
    }
    // 早先派的、盯后台的也记了的，这里已经印过的拿掉。
    if let Some(shell) = ask.shell {
        let turns: Vec<u64> = turn
            .ours
            .into_iter()
            .chain(turn.woken.iter().copied())
            .collect();
        spool::forget(shell, &turn.ended_jobs, &turns);
    }
    // 还有没结束的后台任务：交给盯后台的，回来时印在这个 shell 里。起不来的不要紧，只是不回报。
    let jobs = turn.running_jobs();
    if let (Some(shell), Some(id), false) =
        (ask.shell, turn.transcript.session.clone(), jobs.is_empty())
        && follow::spawn(&follow::Watch {
            session: id,
            jobs,
            shell,
        })
        .is_err()
    {}
    if let Link::Down(reason) = &turn.transcript.link {
        writeln!(io::stderr(), "{reason}")?;
        return Ok(ExitCode::FAILURE);
    }
    Ok(if turn.interrupted {
        ExitCode::from(INTERRUPTED)
    } else {
        ExitCode::SUCCESS
    })
}

/// 读终端界面的配置（语言照环境认）。
///
/// # Errors
///
/// 内置的语言表、配置读不了。
pub fn config() -> io::Result<crate::config::Config> {
    let table = crate::language::LanguageTable::builtin().map_err(io::Error::other)?;
    let language = table.detect(|name| std::env::var(name).ok());
    crate::config::Config::load(&language, true).map_err(io::Error::other)
}

/// 闲置久了换了新会话的那一行：暗色，印在这一轮前面（设计 22 第七节「终端会话」）。
fn renewed_line(since: jiff::Timestamp, words: &Words) -> Line<'static> {
    let ago = terminal::ago(since, jiff::Timestamp::now(), &words.units);
    let text = crate::shellhook::words::fill(&words.renewed, &[("ago", &ago)]);
    Line::from(Span::styled(
        format!("  {text}"),
        Style::default().patch(crate::theme::dim()),
    ))
}
