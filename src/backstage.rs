//! 「软件后台」打开一个软件的后台页（蓝图「配置页」第 43c 条，核心 `package-pages.md`「终端」第 2 条）：在后台跑
//! `<核心程序> web --package <编号> --format json`，拿回 `{url}`，交给系统的打开方式开。核心程序照 `MIYU_CORE_BIN`，再照
//! 核心转交 `miyu tui` 时设的 `MIYU_PROGRAM`；都没有的不跑（不去 `PATH` 里找：装着旧版的机器上那是旧版，旧版不认的子命令
//! 会当成一句话发出去）。跑完照 `wake` 叫醒主循环（同 `service.rs`）。

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

use crate::notify::Wake;

/// 跑完的：哪个包、名字，拿到的网址或者没跑成的那一句。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// 包的编号。
    pub id: String,
    /// 包的名字（状态行用）。
    pub name: String,
    /// 网址；没跑成的是标准错误的第一行（没有的是退出码、没起来的原因）。
    pub url: Result<String, String>,
}

/// 跑哪个核心程序：`MIYU_CORE_BIN`，再是 `MIYU_PROGRAM`；都没有的是 `None`。
pub fn program() -> Option<PathBuf> {
    ["MIYU_CORE_BIN", "MIYU_PROGRAM"].iter().find_map(|name| {
        std::env::var_os(name)
            .filter(|bin| !bin.is_empty())
            .map(|bin| crate::local::expand_home(bin, std::env::home_dir().as_deref()))
    })
}

/// 读 `--format json` 印的那一行：有 `url` 的是它，别的照标准错误的第一行说。
pub fn read(stdout: &str, stderr: &str, status: Option<i32>) -> Result<String, String> {
    let got: Option<serde_json::Value> = stdout
        .lines()
        .rev()
        .find_map(|line| serde_json::from_str(line.trim()).ok());
    if let Some(url) = got.as_ref().and_then(|g| g["url"].as_str())
        && !url.is_empty()
    {
        return Ok(url.to_string());
    }
    let first = stderr.lines().map(str::trim).find(|l| !l.is_empty());
    Err(match (first, status) {
        (Some(line), _) => line.to_string(),
        (None, Some(code)) => format!("exit {code}"),
        (None, None) => "killed".to_string(),
    })
}

/// 在跑的、跑完的。
pub struct Backstage {
    tx: Sender<Reply>,
    rx: Receiver<Reply>,
    /// 跑完叫醒主循环（`main.rs` 装上）。
    wake: Option<Wake>,
}

impl std::fmt::Debug for Backstage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Backstage").finish_non_exhaustive()
    }
}

impl Default for Backstage {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self { tx, rx, wake: None }
    }
}

impl Backstage {
    /// 跑完怎么叫醒主循环。
    pub fn set_wake(&mut self, wake: Wake) {
        self.wake = Some(wake);
    }

    /// 在后台要这个包的后台页网址；不知道核心程序在哪的不跑（按键那里已经说过了）。
    pub fn open(&mut self, id: String, name: String) {
        let Some(program) = program() else {
            return;
        };
        let (tx, wake) = (self.tx.clone(), self.wake.clone());
        std::thread::spawn(move || {
            let url = match std::process::Command::new(&program)
                .args(["web", "--package", &id, "--format", "json"])
                .stdin(std::process::Stdio::null())
                .output()
            {
                Ok(out) => read(
                    &String::from_utf8_lossy(&out.stdout),
                    &String::from_utf8_lossy(&out.stderr),
                    out.status.code(),
                ),
                Err(e) => Err(e.to_string()),
            };
            // 界面已经关了的，跑完也没人收。
            if tx.send(Reply { id, name, url }).is_ok()
                && let Some(wake) = wake
            {
                wake();
            }
        });
    }

    /// 收下跑完的。
    pub fn take(&mut self) -> Option<Reply> {
        self.rx.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::read;

    #[test]
    fn the_url_comes_from_the_json_line_and_a_failure_from_stderr() {
        let ok = read(
            "{\"url\":\"http://127.0.0.1:8770/#package=x\",\"first\":false}\n",
            "",
            Some(0),
        );
        assert_eq!(ok, Ok("http://127.0.0.1:8770/#package=x".into()));
        let bad = read("", "\nminiweb: no such package\nmore\n", Some(2));
        assert_eq!(bad, Err("miniweb: no such package".into()));
        assert_eq!(read("", "", Some(3)), Err("exit 3".into()));
        assert_eq!(read("{\"url\":\"\"}", "", Some(0)), Err("exit 0".into()));
    }
}
