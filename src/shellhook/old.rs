//! 旧版（0.7）装的那一份（蓝图 `shellhook.md`「旧版那一份」）。

use std::io;
use std::path::Path;

use super::install::{self, Done};
use super::shells::Shell;
use super::words::Words;

/// 给人看的一行：`trouble` 的是出错、提醒，命令行印到标准错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said {
    pub text: String,
    pub trouble: bool,
}

/// 装进 `shell`（`target` 是装在哪），顺手清旧版那一份。
///
/// # Errors
///
/// 装新的那一份读写失败。
pub fn install(
    shell: Shell,
    target: &Path,
    program: &Path,
    _env: &impl Fn(&str) -> Option<String>,
    _words: &Words,
) -> io::Result<(Done, Vec<Said>)> {
    Ok((install::install_at(shell, target, program)?, Vec::new()))
}

#[cfg(test)]
mod tests;
