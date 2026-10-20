//! 斜杠命令（设计 22 第七节「斜杠命令」，图纸 `shellhook.md`「斜杠命令」）：和终端界面共用一张表（`resources/commands.json`，
//! `commands::Commands::read` 认），作用在终端会话上。在 fish 里敲 `/model`（`say` 先问这里），或者 `miyu shellhook model`。
//! 表里没有的 `/名字` 照一句话交给核心（核心认的命令模板由它办，同终端界面）。
//!
//! 和终端界面不一样的：`config` 打开终端界面的配置页（`--page config`）；`new` 请核心开一个新的终端会话；开框的几条借终端
//! 界面的框（`--command`，图纸 `tui.md`「只办一条命令」）；`exit`、`help`、`language` 不做。别的还在做。

use std::io::{self, Write};
use std::process::ExitCode;

use serde_json::{Value, json};

use crate::commands::Line;
use crate::config::Config;
use crate::core::{Command, Refusal, Start, Update};
use crate::shellhook::words::{Words, fill};

/// shell 里不做的：退出、帮助、界面语言（设计 22 第七节，2026-10-20 项目主人定）。
const SKIPPED: [&str; 3] = ["exit", "help", "language"];

/// 借终端界面的框办的（2026-10-20 和终端界面会话商定）：开一个框，选完、关了就回 shell。
const BORROWED: [&str; 6] = ["status", "usage", "model", "effort", "theme", "sessions"];

/// 回应等多久：核心没在跑的要先拉起来。
const WAIT: std::time::Duration = std::time::Duration::from_secs(15);

/// `line` 是终端界面的斜杠命令的：办了交回退出码；不是的交回 `None`，照一句话说。
///
/// # Errors
///
/// 读配置、起配置页失败。
pub fn run(line: &str, words: &Words) -> Option<io::Result<ExitCode>> {
    let config = match crate::shellhook::say::config() {
        Ok(config) => config,
        Err(e) => return Some(Err(e)),
    };
    let Line::Command(spec, _args) = config.commands.read(line.trim()) else {
        return None;
    };
    Some(match spec.name.as_str() {
        "config" => config_page(),
        "new" => fresh(&config, words),
        name if BORROWED.contains(&name) => borrow(&config, line.trim()),
        name if SKIPPED.contains(&name) => tell(&fill(&words.slash_skipped, &[("name", name)])),
        name => tell(&fill(&words.slash_pending, &[("name", name)])),
    })
}

/// 打开终端界面的配置页（同一个程序的 `--page config`），退出了回到 shell。
fn config_page() -> io::Result<ExitCode> {
    let status = std::process::Command::new(std::env::current_exe()?)
        .args(["--page", "config"])
        .status()?;
    Ok(exit(status.success()))
}

/// `/new`：请核心照新开终端会话的规矩开一个、指过去（`session.terminal {fresh: true}`，核心 2bd2fa8a）；开在 shell 这时的
/// 目录。下一句就说进新的。
fn fresh(config: &Config, words: &Words) -> io::Result<ExitCode> {
    match ask(
        config,
        "session.terminal",
        json!({ "cwd": cwd(), "fresh": true }),
    )? {
        Ok(_) => {
            writeln!(io::stdout(), "\n  {}\n", words.slash_new)?;
            Ok(ExitCode::SUCCESS)
        }
        Err(refusal) => tell(&refusal.message),
    }
}

/// 借终端界面的框：问到终端会话，起同一个程序 `--resume <它> --command <这一行>`；框关了回来。会话换了（`/sessions` 选了
/// 别的）的，它把新的编号写进交给它的文件，照它指终端会话（`session.set_terminal`）。
fn borrow(config: &Config, line: &str) -> io::Result<ExitCode> {
    let terminal = match ask(config, "session.terminal", json!({ "cwd": cwd() }))? {
        Ok(found) => found["session"].as_str().unwrap_or_default().to_string(),
        Err(refusal) => return tell(&refusal.message),
    };
    let dir = crate::shellhook::say::spool::dir()
        .ok_or_else(|| io::Error::other("no cache directory"))?;
    std::fs::create_dir_all(&dir)?;
    let report = dir.join(format!("report-{}", std::process::id()));
    let status = std::process::Command::new(std::env::current_exe()?)
        .args(["--resume", &terminal, "--command", line, "--report"])
        .arg(&report)
        .status()?;
    let moved = std::fs::read_to_string(&report).ok();
    drop(std::fs::remove_file(&report));
    if let Some(session) = moved.filter(|s| !s.trim().is_empty() && s.trim() != terminal)
        && let Err(refusal) = ask(
            config,
            "session.set_terminal",
            json!({ "session": session.trim() }),
        )?
    {
        return tell(&refusal.message);
    }
    Ok(exit(status.success()))
}

/// 连核心问一次（不进任何会话），交回回应。
fn ask(config: &Config, method: &'static str, params: Value) -> io::Result<Result<Value, Refusal>> {
    let (sender, incoming) = std::sync::mpsc::channel();
    let core = crate::core::spawn(config.layout.reconnect_ms, Start::Bare, move |update| {
        sender.send(update).is_ok()
    });
    core.send(Command::Ask {
        tag: 1,
        method,
        params,
    });
    loop {
        let update = incoming
            .recv_timeout(WAIT)
            .map_err(|_| io::Error::other("no answer from the core"))?;
        match update {
            Update::Answer { tag: 1, result } => return Ok(result),
            Update::NoCoreBin | Update::Failed(_) => {
                return Err(io::Error::other("can't reach the core"));
            }
            _ => {}
        }
    }
}

/// shell 这时的目录。
fn cwd() -> Option<String> {
    std::env::current_dir()
        .ok()
        .map(|d| d.to_string_lossy().into_owned())
}

fn exit(ok: bool) -> ExitCode {
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// 说一句为什么没办，退出码 1。
fn tell(text: &str) -> io::Result<ExitCode> {
    writeln!(io::stderr(), "{text}")?;
    Ok(ExitCode::FAILURE)
}
