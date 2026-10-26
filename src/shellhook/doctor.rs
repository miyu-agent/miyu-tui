//! `miyu shellhook doctor`（设计 22 第七节：给人用的四个命令之一）：一条一条查装好了没有、用得起来没有，有问题的说一句
//! 怎么修。查的是：装进了哪几个 shell（同 `status`）；连得上核心没有；终端是不是真彩色、问不问得到终端的颜色（问不到的
//! 跟随终端只用 16 色）；`Ctrl+V` 读剪贴板的命令在不在。不改任何东西、不开会话。

use std::io::{self, IsTerminal};
use std::process::ExitCode;
use std::time::Duration;

use super::words::{Words, fill};
use crate::core::{Start, Update};

/// 连核心最多等多久：没在跑的要先拉起来。
const CORE_WAIT: Duration = Duration::from_secs(15);

/// 查一遍，印出来；有一条不好的退出码 1。
///
/// # Errors
///
/// 写终端失败、读配置失败。
pub fn run(words: &Words) -> io::Result<ExitCode> {
    super::install::status(words)?;
    let mut good = true;
    let mut say = |ok: bool, text: String| {
        good &= ok;
        let mark = if ok {
            &words.doctor_ok
        } else {
            &words.doctor_bad
        };
        println!("{mark} {text}");
    };
    match core() {
        Ok(true) => say(true, words.doctor_core_ok.clone()),
        Ok(false) => say(false, words.doctor_core_setup.clone()),
        Err(e) => say(
            false,
            fill(&words.doctor_core_bad, &[("reason", &e.to_string())]),
        ),
    }
    let depth = crate::theme::Depth::detect(|name| std::env::var(name).ok());
    let truecolor = depth == crate::theme::Depth::True;
    say(
        truecolor,
        if truecolor {
            words.doctor_truecolor_ok.clone()
        } else {
            words.doctor_truecolor_bad.clone()
        },
    );
    let asked = colors();
    say(
        asked,
        if asked {
            words.doctor_colors_ok.clone()
        } else {
            words.doctor_colors_bad.clone()
        },
    );
    match clipboard() {
        Some(tool) => say(true, fill(&words.doctor_clip_ok, &[("tool", tool)])),
        None => say(
            false,
            fill(
                &words.doctor_clip_bad,
                &[("tools", &crate::clipboard::read_tools().join("/"))],
            ),
        ),
    }
    Ok(if good {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// 连一次核心（不进会话）：头的配置推来了就算连上；还没有机主的（核心 Y-12，只能注册）交回 `false`。
fn core() -> io::Result<bool> {
    let config = super::say::config()?;
    let (sender, incoming) = std::sync::mpsc::channel();
    let _core = crate::core::spawn(config.layout.reconnect_ms, Start::Bare, move |update| {
        sender.send(update).is_ok()
    });
    loop {
        match incoming.recv_timeout(CORE_WAIT) {
            Ok(Update::HeadConfig(_)) => return Ok(true),
            Ok(Update::Account(account)) if account.setup => return Ok(false),
            Ok(Update::NoCoreBin) => return Err(io::Error::other("no core program")),
            Ok(Update::Failed(reason)) => return Err(io::Error::other(reason)),
            Ok(_) => {}
            Err(_) => return Err(io::Error::other("no answer")),
        }
    }
}

/// 问一次终端的字色、底色和 16 色（同 `say` 起来时问的）：都问到了才算。标准输入不是终端的不问。
fn colors() -> bool {
    if !io::stdin().is_terminal() {
        return false;
    }
    if ratatui::crossterm::terminal::enable_raw_mode().is_err() {
        return false;
    }
    let ground = crate::theme::terminal_colors::probe_ground();
    let sixteen = crate::theme::terminal_colors::probe();
    drop(ratatui::crossterm::terminal::disable_raw_mode());
    ground.is_some() && sixteen.is_some()
}

/// `Ctrl+V` 读剪贴板用哪个命令：照 `clipboard::read` 试的那几个、同一个先后（Wayland 下的 `wl-paste`，再 `xclip`、`xsel`；
/// macOS 的 `pbpaste`；Windows 的 `powershell`，2026-10-24 Windows 上原来照 Linux 的查，说找不到）。都没有的是 `None`。
fn clipboard() -> Option<&'static str> {
    crate::clipboard::read_tools()
        .into_iter()
        .find(|tool| on_path(tool))
}

/// `PATH` 里有没有这个命令（Windows 上照 `.exe` 找）。
fn on_path(tool: &str) -> bool {
    let file = if cfg!(windows) {
        format!("{tool}.exe")
    } else {
        tool.to_string()
    };
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(&file).is_file()))
}
