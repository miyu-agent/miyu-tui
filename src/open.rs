//! 用系统的打开方式开一个地址（蓝图 `tui.md`「她的回答：Markdown」第 10 条）：Linux `xdg-open`，
//! macOS `open`，Windows `start`。不等它、不管它的输出；只开 `http`、`https`、`file` 和本机存在的文件，别的协议不碰。
//! 你发出去的附件块也走这里（「输入框」第 12 条）。

use std::io;
use std::process::{Command, Stdio};

/// 开不了的原因（给人看的字在 `text/zh.json` 的 `open`）。
#[derive(Debug)]
pub enum OpenError {
    /// 不开这种协议（`javascript:`、`mailto:` 这些）。
    Scheme,
    /// 本机的文件不在了（挪走、删了）。
    Missing,
    /// 起不来打开它的程序。
    Spawn(io::Error),
}

/// 开 `url`。
///
/// # Errors
///
/// 协议不认、本机的文件不在了、起不来打开它的程序。
pub fn open(url: &str) -> Result<(), OpenError> {
    let target = target(url)?;
    let mut command = opener(&target);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(OpenError::Spawn)?;
    // 另起一个线程等它退出，免得留下僵尸进程；等的结果没人要。
    std::thread::spawn(move || child.wait().is_ok());
    Ok(())
}

/// 交给打开程序的是什么：认得的协议照原样；本机的路径（和图片同一套认法，`local.rs`）换成文件的完整路径，
/// 文件得在；别的协议不开。
fn target(url: &str) -> Result<String, OpenError> {
    if ["http://", "https://", "file://"]
        .iter()
        .any(|s| url.starts_with(s))
    {
        return Ok(url.to_string());
    }
    if has_scheme(url) {
        return Err(OpenError::Scheme);
    }
    let path = crate::local::resolve(url).ok_or(OpenError::Missing)?;
    if path.exists() {
        Ok(path.display().to_string())
    } else {
        Err(OpenError::Missing)
    }
}

/// 开头是「协议:」：字母打头、两个字以上（`C:` 这种盘符不算），只有字母、数字、`+`、`-`、`.`。
fn has_scheme(url: &str) -> bool {
    url.split_once(':').is_some_and(|(head, _)| {
        head.len() > 1
            && head.starts_with(|c: char| c.is_ascii_alphabetic())
            && head
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    })
}

#[cfg(target_os = "macos")]
fn opener(url: &str) -> Command {
    let mut c = Command::new("open");
    c.arg(url);
    c
}

/// Windows：不经过 `cmd`（review，2026-10-11：原来 `cmd /C start "" 地址`，Rust 不给 `&` 加引号，地址里的 `&` 被 cmd 当成
/// 命令分隔，既截断地址，也能跑后面的命令）。交给系统的协议处理程序，地址整个是一个参数。
#[cfg(windows)]
fn opener(url: &str) -> Command {
    let mut c = Command::new("rundll32.exe");
    c.args(["url.dll,FileProtocolHandler", url]);
    c
}

#[cfg(all(unix, not(target_os = "macos")))]
fn opener(url: &str) -> Command {
    let mut c = Command::new("xdg-open");
    c.arg(url);
    c
}

#[cfg(test)]
mod tests {
    use super::open;

    #[cfg(windows)]
    #[test]
    fn an_ampersand_in_a_link_is_not_a_command_separator() {
        let url = "https://x.test/?a=1&calc";
        let c = super::opener(url);
        assert_ne!(c.get_program(), "cmd", "不经过 cmd");
        assert!(c.get_args().any(|a| a == url), "地址整个是一个参数");
    }

    #[test]
    fn other_schemes_are_refused() {
        assert!(open("javascript:alert(1)").is_err());
        assert!(open("ssh://host").is_err());
    }

    #[test]
    fn local_paths_that_exist_are_opened_as_files() {
        use super::{OpenError, target};
        let here = std::env::current_dir().unwrap();
        let file = here.join("Cargo.toml");
        assert_eq!(
            target(file.to_str().unwrap()).ok(),
            Some(file.display().to_string())
        );
        assert_eq!(target("Cargo.toml").ok(), Some(file.display().to_string()));
        assert_eq!(
            target("https://a.com").ok(),
            Some("https://a.com".to_string())
        );
        // 本机的文件不在了、不开的协议：分开说（附件的文件挪走、删了时要说清楚）。
        assert!(matches!(
            target("/没有/这个/文件.png"),
            Err(OpenError::Missing)
        ));
        assert!(matches!(target("mailto:a@b.c"), Err(OpenError::Scheme)));
    }
}
