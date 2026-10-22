//! `shellhook say`：把一句话交给她，这一轮照终端界面时间线的样子印在 shell 里（图纸 `shellhook.md`「say」）。
//!
//! 连核心、收推送、排时间线都照全屏那一套（`core::spawn`、`Transcript`、`ui/rows.rs`），只是不进全屏：光标下面一块活动区
//! 原地刷新（`paint.rs`、`session.rs`），这一轮结束时整段定下来留在滚动区。说到哪个会话问核心（`session.terminal`，
//! `terminal.rs`）。答完还有派出去没结束的后台任务的，起一个盯后台的（`follow.rs`），回报回来时 shell 在提示符上印
//! （`report.rs`）。

mod asking;
mod dreaming;
pub mod follow;
mod look;
mod pack;
mod paint;
mod pick;
pub use pick::run::Act;
pub mod report;
mod session;
mod settings;
pub mod spool;
mod terminal;
mod turn;
mod typing;
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
    /// 这一句是 `/edit` 放回命令行、改过的：替换上一轮重来（`session.redo`，同终端界面的 `/edit`）。
    pub redo: bool,
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
    // `/redo`：上一轮原样重来，照说话那样画重来的那一轮。
    let bare_redo = text.trim() == "/redo";
    // 终端界面的斜杠命令（`/models`）：照命令办，不当一句话（`shellhook/slash.rs`）。
    if text.starts_with('/')
        && !bare_redo
        && let Some(done) =
            crate::shellhook::slash::run(&text, words, ask.leftover.as_deref(), ask.shell)
    {
        return done;
    }
    // 这一句里 `Ctrl+V` 贴的块：换回原文、带上附件（`shellhook/paste.rs`）。
    let (text, files) = match ask.shell {
        Some(shell) => crate::shellhook::paste::take(shell, &text),
        None => (text, Vec::new()),
    };
    let (mut session, look) = Session::start()?;
    session.hooked = ask.shell.is_some();
    // 这一句照 shell 所在的目录说（设计 22 第七节「她在哪个目录干活」）。
    let cwd = std::env::current_dir()
        .ok()
        .map(|d| d.to_string_lossy().into_owned());
    let turn_text = text.clone();
    let mut turn = Turn::new(text, cwd, look);
    turn.files = files;
    turn.redo = ask.redo;
    turn.bare_redo = bare_redo;
    // 终端界面不认的斜杠命令（核心的命令模板、扩展的、`/dream`）：整行交给核心（同终端界面 `app/catalog.rs`）。
    // `/compact`：照说话那样画压缩那一轮。
    turn.compact = crate::shellhook::slash::compact(&session.config.commands, &turn_text);
    turn.slash = !bare_redo
        && turn_text.starts_with('/')
        && crate::shellhook::slash::to_core(&session.config.commands, &turn_text);
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

/// 斜杠命令在 shell 里原地选的那几种（`pick.rs`）。
pub enum Pick<'a> {
    /// `/model`，带着后面的字。
    Model(Option<&'a str>),
    /// `/theme`。
    Theme(Option<&'a str>),
    /// `/effort`。
    Effort(Option<&'a str>),
    /// `/sessions`。
    Sessions,
    /// `/status`。
    Status,
    /// `/persona`、`/preset`。
    Persona(crate::app::new_session::Kind, Option<&'a str>),
    /// `/usage`。
    Usage,
    /// `/new`。
    Fresh,
    /// `/help`。
    Help,
    /// `/undo`、`/restore`、`/recap`。
    Run(pick::run::Act),
    /// `/edit`：上一句写进这个文件，脚本放回命令行。
    /// 放回命令行的文件、哪个 shell（贴过的块照它记着的换回块上的字）。
    Edit(Option<&'a std::path::Path>, Option<u32>),
}

/// 在提示符下面原地画一个列表，定了留一行（`pick.rs`，图纸「斜杠命令」第 3 条）。
///
/// # Errors
///
/// 读配置、开原始模式、写终端失败。
pub fn pick(which: Pick<'_>, words: &Words, hooked: bool) -> io::Result<ExitCode> {
    let (mut session, look) = Session::start()?;
    session.hooked = hooked;
    let cwd = std::env::current_dir()
        .ok()
        .map(|d| d.to_string_lossy().into_owned());
    let mut turn = Turn::new(String::new(), cwd, look);
    let mut picker: Box<dyn pick::Picker> = match which {
        Pick::Model(given) => Box::new(pick::model::ModelPick::new(given, words)),
        Pick::Theme(given) => Box::new(pick::theme::ThemePick::new(given, words)),
        Pick::Effort(given) => Box::new(pick::effort::EffortPick::new(given, words)),
        Pick::Sessions => Box::new(pick::sessions::SessionsPick::new(words)),
        Pick::Status => Box::new(pick::status::StatusShow::new()),
        Pick::Usage => Box::new(pick::usage::UsageShow::new()),
        Pick::Fresh => Box::new(pick::fresh::FreshPick::new(words)),
        Pick::Help => Box::new(pick::help::HelpShow),
        Pick::Run(act) => Box::new(pick::run::RunPick::new(act, words)),
        Pick::Edit(leftover, shell) => Box::new(pick::edit::EditPick::new(leftover, shell)),
        Pick::Persona(kind, given) => Box::new(pick::persona::PersonaPick::new(
            kind,
            given,
            words,
            &session.config,
        )),
    };
    session.pick(&mut turn, picker.as_mut())?;
    session.finish()?;
    Ok(ExitCode::SUCCESS)
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
