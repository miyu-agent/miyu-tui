//! shell 起来时现印的脚本（`shellhook script`）：装进 shell 的只有调它的一行，升级以后自动是新的（设计 22 第七节）。
//!
//! 已经开着的 shell 留着起来时印的那一份：升级以后旧脚本调新程序。脚本调程序时头上带 `--hook <记号>`（[`stamp`]），
//! 程序看记号对不上的就是脚本变过了，说一声重开终端（`mod.rs`）。

use std::path::Path;

use super::shells::Shell;

/// 脚本里换成程序的那个记号。
const PROGRAM: &str = "@PROGRAM@";

/// fish、bash、zsh 的脚本；PowerShell 两种共用一份。
const SOURCES: [&str; 3] = [
    include_str!("../../resources/shellhook/hook.fish"),
    include_str!("../../resources/shellhook/hook.bash"),
    include_str!("../../resources/shellhook/hook.zsh"),
];
const POWERSHELL: &str = include_str!("../../resources/shellhook/hook.ps1");

/// 这一版的脚本的记号：四份脚本原文的 FNV-1a，哪一份改了都变（只看程序版本的话，脚本没变的升级也要人重开）。
pub fn stamp() -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in SOURCES.iter().chain([&POWERSHELL]).flat_map(|s| s.bytes()) {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// 这种 shell 的脚本，调 `program`（装的时候记下的那个）。
pub fn render(shell: Shell, program: &Path) -> String {
    let source = match shell {
        Shell::Fish => SOURCES[0],
        Shell::Bash => SOURCES[1],
        Shell::Zsh => SOURCES[2],
        Shell::Pwsh | Shell::WinPs => POWERSHELL,
    };
    let text = source
        .replace(PROGRAM, &shell.quote(program))
        .replace("@HOOK@", &stamp());
    if !shell.powershell() {
        return text;
    }
    // PowerShell 没有信号可等：画提示符时自己看信箱（`say/spool.rs`），这里告诉它在哪。认不出的写一个不在的地方，不印回报。
    let spool = super::say::spool::dir()
        .unwrap_or_else(|| std::env::temp_dir().join("miyu-shellhook-none"));
    text.replace("@SHELL@", shell.name())
        .replace("@SPOOL@", &shell.quote(&spool))
}
