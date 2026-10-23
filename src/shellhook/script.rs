//! shell 起来时现印的脚本（`shellhook script`）：装进 shell 的只有调它的一行，升级以后自动是新的（设计 22 第七节）。

use std::path::Path;

use super::shells::Shell;

/// 脚本里换成程序的那个记号。
const PROGRAM: &str = "@PROGRAM@";

/// 这种 shell 的脚本，调 `program`（装的时候记下的那个）。
pub fn render(shell: Shell, program: &Path) -> String {
    let source = match shell {
        Shell::Fish => include_str!("../../resources/shellhook/hook.fish"),
        Shell::Bash => include_str!("../../resources/shellhook/hook.bash"),
        Shell::Zsh => include_str!("../../resources/shellhook/hook.zsh"),
        Shell::Pwsh | Shell::WinPs => include_str!("../../resources/shellhook/hook.ps1"),
    };
    let text = source.replace(PROGRAM, &shell.quote(program));
    if !shell.powershell() {
        return text;
    }
    // PowerShell 没有信号可等：画提示符时自己看信箱（`say/spool.rs`），这里告诉它在哪。认不出的写一个不在的地方，不印回报。
    let spool = super::say::spool::dir()
        .unwrap_or_else(|| std::env::temp_dir().join("miyu-shellhook-none"));
    text.replace("@SHELL@", shell.name())
        .replace("@SPOOL@", &shell.quote(&spool))
}
