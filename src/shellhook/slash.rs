//! 斜杠命令（设计 22 第七节「斜杠命令」，图纸 `shellhook.md`「斜杠命令」）：和终端界面共用一张表（`resources/commands.json`，
//! `commands::Commands::read` 认），作用在终端会话上。在 fish 里敲 `/models`（`say` 先问这里），或者 `miyu shellhook models`。
//! 表里没有的 `/名字` 照一句话交给核心（核心认的命令模板由它办，同终端界面）。
//!
//! 和终端界面不一样的：`config` 打开终端界面的配置页（`--page config`）；`exit`、`help`、`language` 不做。别的还在做。

use std::io::{self, Write};
use std::process::ExitCode;

use crate::commands::Line;
use crate::shellhook::words::{Words, fill};

/// shell 里不做的：退出、帮助、界面语言（设计 22 第七节，2026-10-20 项目主人定）。
const SKIPPED: [&str; 3] = ["exit", "help", "language"];

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
        name if SKIPPED.contains(&name) => tell(&fill(&words.slash_skipped, &[("name", name)])),
        name => tell(&fill(&words.slash_pending, &[("name", name)])),
    })
}

/// 打开终端界面的配置页（同一个程序的 `--page config`），退出了回到 shell。
fn config_page() -> io::Result<ExitCode> {
    let status = std::process::Command::new(std::env::current_exe()?)
        .args(["--page", "config"])
        .status()?;
    Ok(if status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// 说一句为什么没办，退出码 1。
fn tell(text: &str) -> io::Result<ExitCode> {
    writeln!(io::stderr(), "{text}")?;
    Ok(ExitCode::FAILURE)
}
