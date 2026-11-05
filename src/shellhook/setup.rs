//! 给终端界面的第一次引导用（2026-10-23 项目主人：引导里加一步「是否无缝集成至终端」）：这台机器上有哪几种 shell、
//! 装没装，装、删。同一个程序里直接调，不走命令行、不走核心；装、删照 `shellhook install`、`uninstall` 那一套，只是
//! 不往标准输出印，结果交回去由界面画。

use std::path::PathBuf;

use super::install::{self, Done, Found};
use super::shells::{self, Shell};
use super::words::{Words, fill};

/// 一种 shell。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// 名字：`fish`、`bash`、`zsh`，装、删时照它认。
    pub name: &'static str,
    /// 已经装好了（自己写的那一段在；记的程序不在了的也算装了，再装一次换成现在的）。
    pub installed: bool,
}

/// 这台机器上有的 shell：PATH 上找得到的，和找不到却已经装过的（删得掉）。照 `status` 的先后。
pub fn shells() -> Vec<Entry> {
    let env = |name: &str| std::env::var(name).ok();
    let path = std::env::var_os("PATH");
    shells::ALL
        .into_iter()
        .filter_map(|shell| {
            let installed = shell
                .target(&env)
                .and_then(|target| install::found(shell, &target).ok())
                .is_some_and(|found| matches!(found, Found::Ours(_)));
            (installed || on_path(shell.name(), path.as_deref())).then_some(Entry {
                name: shell.name(),
                installed,
            })
        })
        .collect()
}

/// 装进 `name` 这种 shell，调这个程序旁边的 `miyu`（同 `shellhook install` 不带 `--program`）。
///
/// # Errors
///
/// 认不出的名字、没有家目录、读写失败、fish 那个文件不是自己写的、PowerShell 的执行策略不让加载（这时已经装上了）：
/// 交回给人看的一句。
pub fn install(name: &str) -> Result<(), String> {
    let words = Words::detect().map_err(|e| e.to_string())?;
    let (shell, target) = locate(name, &words)?;
    let program = install::default_program().map_err(|e| e.to_string())?;
    match install::install_at(shell, &target, &program).map_err(|e| e.to_string())? {
        // 装上了，可 PowerShell 的执行策略不让加载启动文件：照失败说，告诉人怎么改（不替人改安全设置）。
        Done::Installed | Done::Updated => match install::blocking_policy(shell) {
            Some(policy) => Err(fill(&words.ps_policy, &[("policy", &policy)])),
            None => Ok(()),
        },
        Done::Foreign => {
            let shown = target.display().to_string();
            Err(fill(
                &words.foreign,
                &[("path", shown.as_str()), ("shell", shell.name())],
            ))
        }
    }
}

/// 从 `name` 这种 shell 里删掉；本来就没装的也算成了。
///
/// # Errors
///
/// 认不出的名字、没有家目录、读写失败：交回给人看的一句。
pub fn uninstall(name: &str) -> Result<(), String> {
    let words = Words::detect().map_err(|e| e.to_string())?;
    let (shell, target) = locate(name, &words)?;
    install::uninstall_at(shell, &target)
        .map(drop)
        .map_err(|e| e.to_string())
}

/// 名字认成 shell、找到装在哪。
fn locate(name: &str, words: &Words) -> Result<(Shell, PathBuf), String> {
    let shell = Shell::named(name).ok_or_else(|| fill(&words.unknown_shell, &[("shell", name)]))?;
    let env = |key: &str| std::env::var(key).ok();
    let target = shell
        .target(&env)
        .ok_or_else(|| install::no_home().to_string())?;
    Ok((shell, target))
}

/// PATH 上有没有叫这个名字的程序（Windows 上照 `.exe` 找）。
fn on_path(name: &str, path: Option<&std::ffi::OsStr>) -> bool {
    let file = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    path.is_some_and(|path| std::env::split_paths(path).any(|dir| dir.join(&file).is_file()))
}

#[cfg(test)]
mod tests {
    use super::on_path;

    #[test]
    fn a_shell_counts_only_when_it_is_on_the_path() {
        let dir = std::env::temp_dir().join(format!("miyu-shellhook-setup-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = if cfg!(windows) { "zsh.exe" } else { "zsh" };
        std::fs::write(dir.join(file), b"").unwrap();
        assert!(on_path("zsh", Some(dir.as_os_str())));
        assert!(!on_path("fish", Some(dir.as_os_str())));
        assert!(!on_path("zsh", None));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
