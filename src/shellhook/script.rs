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
    };
    source.replace(PROGRAM, &shell.quote(program))
}
