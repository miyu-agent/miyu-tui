//! 用程序开一个本机的文件（蓝图 `tui.md`「展示」第 7 条，2026-10-17 项目主人照推荐定）：先交系统给这种文件定的默认
//! 程序；没有的交浏览器；浏览器也没有的交回 [`Launched::Nothing`]，由界面写「没有能打开它的程序」和副本的路径。
//! 不等打开它的程序、不管它的输出。

use std::path::Path;
use std::process::{Command, Stdio};

/// 交给谁开了。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Launched {
    /// 系统的默认程序。
    Default,
    /// 浏览器。
    Browser,
    /// 没有能打开它的程序。
    Nothing,
}

/// Linux 上没设 `$BROWSER` 时挨个试的浏览器。
#[cfg(all(unix, not(target_os = "macos")))]
const BROWSERS: [&str; 7] = [
    "firefox",
    "chromium",
    "chromium-browser",
    "google-chrome",
    "google-chrome-stable",
    "brave-browser",
    "microsoft-edge",
];

/// 找程序的地方：生产照这个进程的 `PATH`、`$BROWSER`，测试换成假的。
#[derive(Debug, Clone, Default)]
pub struct Env {
    /// 找程序的 `PATH`；`None` 照这个进程的。
    pub path: Option<std::ffi::OsString>,
    /// `$BROWSER`；`None` 是没设。
    pub browser: Option<String>,
}

impl Env {
    /// 这个进程的。
    pub fn current() -> Env {
        Env {
            path: None,
            browser: std::env::var("BROWSER").ok(),
        }
    }

    /// 一条命令：照这里的 `PATH` 找程序。
    fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        if let Some(path) = &self.path {
            command.env("PATH", path);
        }
        command
    }
}

/// 开 `path`（`media_type` 是核心认的种类）。
pub fn launch(path: &Path, media_type: &str, env: &Env) -> Launched {
    if default_program(path, media_type, env) {
        return Launched::Default;
    }
    if browser(path, env) {
        return Launched::Browser;
    }
    Launched::Nothing
}

/// 起一个程序，不等它（另起一个线程收尸）；起不来是 `false`。
fn spawn(mut command: Command) -> bool {
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match child {
        Ok(mut child) => {
            std::thread::spawn(move || child.wait().is_ok());
            true
        }
        Err(_) => false,
    }
}

/// `$BROWSER` 里写的：冒号隔开的几个，第一个起得来的算；带 `%s` 的把它换成路径。
fn from_env(path: &Path, env: &Env) -> bool {
    let Some(list) = &env.browser else {
        return false;
    };
    let target = path.display().to_string();
    list.split(':')
        .filter(|s| !s.trim().is_empty())
        .any(|entry| {
            let mut words = entry.split_whitespace();
            let Some(program) = words.next() else {
                return false;
            };
            let mut command = env.command(program);
            let mut placed = false;
            for word in words {
                if word.contains("%s") {
                    command.arg(word.replace("%s", &target));
                    placed = true;
                } else {
                    command.arg(word);
                }
            }
            if !placed {
                command.arg(&target);
            }
            spawn(command)
        })
}

#[cfg(all(unix, not(target_os = "macos")))]
fn default_program(path: &Path, media_type: &str, env: &Env) -> bool {
    // 先问这种文件有没有默认程序：`xdg-open` 没有也照样退出 0 以外的码，不等它就分不出来。
    let known = env
        .command("xdg-mime")
        .args(["query", "default", media_type])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|out| {
            out.status.success() && !String::from_utf8_lossy(&out.stdout).trim().is_empty()
        });
    if !known {
        return false;
    }
    let mut command = env.command("xdg-open");
    command.arg(path);
    spawn(command)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn browser(path: &Path, env: &Env) -> bool {
    from_env(path, env)
        || BROWSERS.iter().any(|name| {
            let mut command = env.command(name);
            command.arg(path);
            spawn(command)
        })
}

#[cfg(target_os = "macos")]
fn default_program(path: &Path, _media_type: &str, env: &Env) -> bool {
    // `open` 找不到能开它的程序时退出码不是 0：等它（一般不到十分之一秒）。
    env.command("open")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

#[cfg(target_os = "macos")]
fn browser(path: &Path, env: &Env) -> bool {
    from_env(path, env)
        || env
            .command("open")
            .args(["-a", "Safari"])
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
}

#[cfg(windows)]
fn default_program(path: &Path, _media_type: &str, env: &Env) -> bool {
    // 交给系统的处理程序（同 `open.rs`）：分不出有没有默认程序，交出去就算开了。
    let mut command = env.command("rundll32.exe");
    command.arg("url.dll,FileProtocolHandler").arg(path);
    spawn(command)
}

#[cfg(windows)]
fn browser(path: &Path, env: &Env) -> bool {
    from_env(path, env)
}

#[cfg(test)]
mod tests;
