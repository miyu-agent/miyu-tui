//! 斜杠命令（设计 22 第七节「斜杠命令」，图纸 `shellhook.md`「斜杠命令」）：和终端界面共用一张表（`resources/commands.json`，
//! `commands::Commands::read` 认），作用在终端会话上。在 fish 里敲 `/models`（`say` 先问这里），或者 `miyu shellhook models`。
//! 表里没有的 `/名字` 照一句话交给核心（核心认的命令模板由它办，同终端界面）。
//!
//! 和终端界面不一样的：`config` 打开终端界面的配置页（`--page config`）；`exit`、`help`、`language` 不做。别的还在做。

use std::io::{self, Write};
use std::process::ExitCode;

use crate::commands::Line;
use crate::shellhook::words::{Words, fill};

/// shell 里不做的：退出、界面语言（设计 22 第七节，2026-10-20 项目主人定；帮助 2026-10-21 改成照终端界面印出来）。
const SKIPPED: [&str; 2] = ["exit", "language"];

/// `line` 是终端界面的斜杠命令的：办了交回退出码；不是的交回 `None`，照一句话说。
///
/// # Errors
///
/// 读配置、起配置页失败。
pub fn run(
    line: &str,
    words: &Words,
    leftover: Option<&std::path::Path>,
    hooked: bool,
) -> Option<io::Result<ExitCode>> {
    let config = match crate::shellhook::say::config() {
        Ok(config) => config,
        Err(e) => return Some(Err(e)),
    };
    let line = line.trim();
    // shell 里才有的：用终端界面打开终端会话（2026-10-21 项目主人：「有没有可能，用 TUI 方式进入这个终端集成会话」）。
    if line == "/tui" {
        return Some(open_tui(&config));
    }
    let (spec, args) = match config.commands.read(line) {
        Line::Command(spec, args) => (spec, args),
        // 终端界面不收参数、shell 里收的（`/model 名字`：不弹列表直接换）。
        Line::Talk => {
            let name = crate::commands::typed(line)?;
            let spec = config
                .commands
                .find(name)
                .filter(|s| WITH_ARGS.contains(&s.name.as_str()))?;
            let rest = line[1 + name.len()..].trim();
            (spec, (!rest.is_empty()).then_some(rest))
        }
        Line::Unknown => return None,
    };
    Some(match spec.name.as_str() {
        "config" => config_page(),
        "help" => crate::shellhook::say::pick(crate::shellhook::say::Pick::Help, words, hooked),
        "new" => crate::shellhook::say::pick(crate::shellhook::say::Pick::Fresh, words, hooked),
        "model" => {
            crate::shellhook::say::pick(crate::shellhook::say::Pick::Model(args), words, hooked)
        }
        "theme" => {
            crate::shellhook::say::pick(crate::shellhook::say::Pick::Theme(args), words, hooked)
        }
        "effort" => {
            crate::shellhook::say::pick(crate::shellhook::say::Pick::Effort(args), words, hooked)
        }
        "sessions" => {
            crate::shellhook::say::pick(crate::shellhook::say::Pick::Sessions, words, hooked)
        }
        "status" => crate::shellhook::say::pick(crate::shellhook::say::Pick::Status, words, hooked),
        "usage" => crate::shellhook::say::pick(crate::shellhook::say::Pick::Usage, words, hooked),
        "edit" => {
            crate::shellhook::say::pick(crate::shellhook::say::Pick::Edit(leftover), words, hooked)
        }
        "persona" => crate::shellhook::say::pick(
            crate::shellhook::say::Pick::Persona(crate::app::new_session::Kind::Persona, args),
            words,
            hooked,
        ),
        "preset" => crate::shellhook::say::pick(
            crate::shellhook::say::Pick::Persona(crate::app::new_session::Kind::Preset, args),
            words,
            hooked,
        ),
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

/// shell 里能接参数、不弹列表直接改的（2026-10-20 项目主人：「直接通过 cli 改也行啊」）。
const WITH_ARGS: [&str; 5] = ["model", "theme", "effort", "persona", "preset"];

/// `/tui`：问到终端会话，起同一个程序 `--resume` 它（全屏的终端界面），退出了回到 shell。
fn open_tui(config: &crate::config::Config) -> io::Result<ExitCode> {
    let (sender, incoming) = std::sync::mpsc::channel();
    let core = crate::core::spawn(
        config.layout.reconnect_ms,
        crate::core::Start::Bare,
        move |update| sender.send(update).is_ok(),
    );
    let cwd = std::env::current_dir()
        .ok()
        .map(|d| d.to_string_lossy().into_owned());
    core.send(crate::core::Command::Ask {
        tag: 1,
        method: "session.terminal",
        params: serde_json::json!({ "cwd": cwd }),
    });
    let session = loop {
        match incoming.recv_timeout(std::time::Duration::from_secs(15)) {
            Ok(crate::core::Update::Answer { tag: 1, result }) => match result {
                Ok(found) => break found["session"].as_str().unwrap_or_default().to_string(),
                Err(refusal) => return tell(&refusal.message),
            },
            Ok(crate::core::Update::NoCoreBin | crate::core::Update::Failed(_)) | Err(_) => {
                return Err(io::Error::other("can't reach the core"));
            }
            Ok(_) => {}
        }
    };
    drop(core);
    let status = std::process::Command::new(std::env::current_exe()?)
        .args(["--resume", &session])
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
