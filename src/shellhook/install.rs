//! 装进 shell、删掉、看装没装（设计 22 第七节）：fish 是 conf.d 下自己的一个文件，bash、zsh 是启动文件里带记号的一段；
//! 装进去的只有调 `shellhook script` 的一行，升级以后自动是新的。不是自己写的文件不动（设计 27）；启动文件是符号链接的
//! 顺着链接写，不把链接换成普通文件（旧版出过这个事）。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::shells::{self, Shell};
use super::words::{Words, fill};

/// fish 那个文件的第一行、bash 和 zsh 那一段的开头：认它是不是自己写的。
const MARK: &str = "# >>> miyu shellhook >>>";
/// bash、zsh 那一段的结尾。
const END: &str = "# <<< miyu shellhook <<<";

/// 装好了的一份是什么样。
#[derive(Debug, PartialEq, Eq)]
pub enum Found {
    /// 没装。
    Missing,
    /// 自己写的，调的是这个程序（读不出来的是 `None`）。
    Ours(Option<PathBuf>),
    /// fish 的那个文件在，却不是自己写的。
    Foreign,
}

/// 装完是哪一种。
#[derive(Debug, PartialEq, Eq)]
pub enum Done {
    /// 新装的。
    Installed,
    /// 原来就有，换成了这一份。
    Updated,
    /// fish 的文件不是自己写的，没动。
    Foreign,
}

/// 装进 `shell`：调 `program`，没给的照默认（[`default_program`]）。
///
/// # Errors
///
/// 读写文件失败、找不到程序。
pub fn install(shell: Shell, program: Option<PathBuf>, words: &Words) -> io::Result<()> {
    let env = |name: &str| std::env::var(name).ok();
    let path = shell.target(&env).ok_or_else(no_home)?;
    let program = match program {
        Some(program) => std::path::absolute(program)?,
        None => default_program()?,
    };
    if !program.is_file() {
        let program = program.display().to_string();
        return Err(io::Error::other(fill(
            &words.no_program,
            &[("program", &program)],
        )));
    }
    let done = install_at(shell, &path, &program)?;
    let shown = path.display().to_string();
    let values = [("path", shown.as_str()), ("shell", shell.name())];
    match done {
        Done::Installed => println!("{}", fill(&words.installed, &values)),
        Done::Updated => println!("{}", fill(&words.updated, &values)),
        Done::Foreign => return Err(io::Error::other(fill(&words.foreign, &values))),
    }
    warn_old(shell, &env, words);
    Ok(())
}

/// 从 `shell` 里删掉。
///
/// # Errors
///
/// 读写文件失败。
pub fn uninstall(shell: Shell, words: &Words) -> io::Result<()> {
    let env = |name: &str| std::env::var(name).ok();
    let path = shell.target(&env).ok_or_else(no_home)?;
    let shown = path.display().to_string();
    let values = [("path", shown.as_str()), ("shell", shell.name())];
    if uninstall_at(shell, &path)? {
        println!("{}", fill(&words.removed, &values));
    } else {
        println!("{}", fill(&words.not_installed, &values));
    }
    Ok(())
}

/// 三种 shell 各装没装，一行一种；旧版的那一份也在的说一声。
///
/// # Errors
///
/// 读文件失败（不在的不算）。
pub fn status(words: &Words) -> io::Result<()> {
    let env = |name: &str| std::env::var(name).ok();
    for shell in shells::ALL {
        let Some(path) = shell.target(&env) else {
            continue;
        };
        let shown = path.display().to_string();
        let mut values = vec![("path", shown.as_str()), ("shell", shell.name())];
        let found = found(shell, &path)?;
        let ours = matches!(found, Found::Ours(_));
        let line = match found {
            Found::Missing => fill(&words.status_missing, &values),
            Found::Foreign => fill(&words.status_foreign, &values),
            Found::Ours(Some(program)) if !program.is_file() => {
                let program = program.display().to_string();
                values.push(("program", &program));
                fill(&words.status_stale, &values)
            }
            Found::Ours(_) => fill(&words.status_installed, &values),
        };
        println!("{line}");
        if ours {
            warn_old(shell, &env, words);
        }
    }
    Ok(())
}

/// 默认调哪个程序：这个程序旁边的 `miyu`（随 Miyu 装的界面放在它旁边，设计 32 第六节），没有的调自己。
///
/// # Errors
///
/// 认不出自己在哪。
pub fn default_program() -> io::Result<PathBuf> {
    let me = std::env::current_exe()?;
    let miyu = me.with_file_name(if cfg!(windows) { "miyu.exe" } else { "miyu" });
    Ok(if miyu.is_file() { miyu } else { me })
}

/// 装进去的那一段。
pub fn block(shell: Shell, program: &Path) -> String {
    let p = shell.quote(program);
    let note = "# `miyu shellhook install` 写的，`miyu shellhook uninstall` 删";
    match shell {
        Shell::Fish => format!(
            "{MARK}\n{note}\nif status is-interactive; and test -x {p}\n    {p} shellhook script --shell fish --program {p} | source\nend\n"
        ),
        Shell::Bash => format!(
            "{MARK}\n{note}\nif [[ $- == *i* ]] && [ -x {p} ]; then eval \"$({p} shellhook script --shell bash --program {p})\"; fi\n{END}\n"
        ),
        Shell::Zsh => format!(
            "{MARK}\n{note}\nif [[ -o interactive && -x {p} ]]; then eval \"$({p} shellhook script --shell zsh --program {p})\"; fi\n{END}\n"
        ),
    }
}

/// 装进 `path`。
///
/// # Errors
///
/// 读写失败。
pub fn install_at(shell: Shell, path: &Path, program: &Path) -> io::Result<Done> {
    let block = block(shell, program);
    let existing = read(path)?;
    let (text, done) = match (shell, existing) {
        (Shell::Fish, None) => (block, Done::Installed),
        (Shell::Fish, Some(old)) if old.starts_with(MARK) => (block, Done::Updated),
        (Shell::Fish, Some(_)) => return Ok(Done::Foreign),
        (_, None) => (block, Done::Installed),
        (_, Some(old)) => match cut(&old) {
            Some((before, after)) => (format!("{before}{block}{after}"), Done::Updated),
            None => {
                let gap = if old.is_empty() || old.ends_with('\n') {
                    ""
                } else {
                    "\n"
                };
                (format!("{old}{gap}{block}"), Done::Installed)
            }
        },
    };
    write(path, &text)?;
    Ok(done)
}

/// 从 `path` 删掉：删了的回 `true`，本来就没有的回 `false`。fish 不是自己写的文件也算没有。
///
/// # Errors
///
/// 读写失败。
pub fn uninstall_at(shell: Shell, path: &Path) -> io::Result<bool> {
    let Some(old) = read(path)? else {
        return Ok(false);
    };
    match shell {
        Shell::Fish if old.starts_with(MARK) => {
            fs::remove_file(path)?;
            Ok(true)
        }
        Shell::Fish => Ok(false),
        Shell::Bash | Shell::Zsh => match cut(&old) {
            Some((before, after)) => {
                write(path, &format!("{before}{after}"))?;
                Ok(true)
            }
            None => Ok(false),
        },
    }
}

/// `path` 里装的是什么样。
///
/// # Errors
///
/// 读失败（不在的不算）。
pub fn found(shell: Shell, path: &Path) -> io::Result<Found> {
    let Some(text) = read(path)? else {
        return Ok(Found::Missing);
    };
    let ours = match shell {
        Shell::Fish if text.starts_with(MARK) => Some(text.as_str()),
        Shell::Fish => return Ok(Found::Foreign),
        Shell::Bash | Shell::Zsh => cut(&text).map(|(before, _)| &text[before.len()..]),
    };
    Ok(match ours {
        Some(ours) => Found::Ours(program_in(ours)),
        None => Found::Missing,
    })
}

/// 装进去的那一段里记的程序：`test -x` 或 `-x` 后面那个单引号包着的词。
fn program_in(block: &str) -> Option<PathBuf> {
    // fish 的单引号里 \' \\ 是转义；POSIX 的引号里写不了引号，写成 '\''（[`Shell::quote`]）。
    let fish = block.contains("status is-interactive");
    let quoted = &block[block.find("-x '")? + "-x '".len()..];
    let mut out = String::new();
    let mut at = 0;
    while let Some(rest) = quoted.get(at..) {
        if fish && rest.starts_with('\\') {
            let c = rest[1..].chars().next()?;
            out.push(c);
            at += 1 + c.len_utf8();
        } else if !fish && rest.starts_with("'\\''") {
            out.push('\'');
            at += "'\\''".len();
        } else if rest.starts_with('\'') {
            return Some(PathBuf::from(out));
        } else {
            let c = rest.chars().next()?;
            out.push(c);
            at += c.len_utf8();
        }
    }
    None
}

/// 启动文件里自己那一段切出来：前面的、后面的（不含那一段）。没有的是 `None`。
fn cut(text: &str) -> Option<(&str, &str)> {
    let start = text
        .match_indices(MARK)
        .map(|(i, _)| i)
        .find(|&i| i == 0 || text[..i].ends_with('\n'))?;
    let end_at = text[start..].find(END)? + start + END.len();
    let end = text[end_at..]
        .strip_prefix('\n')
        .map_or(end_at, |_| end_at + 1);
    Some((&text[..start], &text[end..]))
}

/// 读一个文件；不在的是 `None`。
fn read(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// 写一个文件：顺着符号链接写到它指的那个，先写在旁边再换过去，权限照原来的。
fn write(path: &Path, text: &str) -> io::Result<()> {
    let target = real(path)?;
    if let Some(dir) = target.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(".miyu-shellhook.tmp");
    let temp = target.with_file_name(name);
    fs::write(&temp, text)?;
    if let Ok(meta) = fs::metadata(&target) {
        fs::set_permissions(&temp, meta.permissions())?;
    }
    fs::rename(&temp, &target)
}

/// 符号链接指的那个文件（一层层跟到底，指着不在的文件也照它）；不是链接的照原样。
fn real(path: &Path) -> io::Result<PathBuf> {
    let mut path = path.to_path_buf();
    for _ in 0..40 {
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                let link = fs::read_link(&path)?;
                path = match path.parent() {
                    Some(dir) if link.is_relative() => dir.join(link),
                    _ => link,
                };
            }
            _ => return Ok(path),
        }
    }
    Err(io::Error::other(format!(
        "{}: too many symlinks",
        path.display()
    )))
}

/// 旧版那一份也在：说一声（不删，不是自己写的）。
fn warn_old(shell: Shell, env: &impl Fn(&str) -> Option<String>, words: &Words) {
    if let Some(old) = shell.old_hook(env).filter(|p| p.exists()) {
        let shown = old.display().to_string();
        eprintln!("{}", fill(&words.old_hook, &[("path", &shown)]));
    }
}

pub fn no_home() -> io::Error {
    io::Error::other("HOME is not set")
}
