//! 开机启动核心（蓝图 `tui.md`「配置页」的「开机启动」，核心 Z-1 下的 `miyu service enable|disable|status --format json`）：
//! 在后台跑核心程序的这条命令（一次几十毫秒到一两秒，`systemctl` 要等），读它印的一行 JSON；跑完叫醒主循环，不每一拍醒来看
//! （蓝图「空闲不醒」）。程序只照 `MIYU_CORE_BIN`（终端拉起的那一个）：服务里写的是跑的这个程序的真实位置，要和终端连的
//! 是同一个；没设的不跑、配置页不列这一行。不找 `PATH` 上的 `miyu`：那可能是旧版，旧版把认不得的子命令当一句话发给正在
//! 跑的旧核心（2026-10-23 本机 `PATH` 上就是旧版 0.7.4）。

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

use serde_json::Value;

use crate::theme::wallpaper::Wake;

/// 要做的：问、装上、拿掉。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Status,
    Enable,
    Disable,
}

impl Verb {
    fn word(self) -> &'static str {
        match self {
            Verb::Status => "status",
            Verb::Enable => "enable",
            Verb::Disable => "disable",
        }
    }
}

/// 一次的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// 成了：装没装、服务管理器那边开没开着（问不了的是 `None`）、写的那个文件。
    Done {
        verb: Verb,
        installed: bool,
        on: Option<bool>,
        file: String,
    },
    /// 没成：核心的代码（`data_root_on_windows`、`no_home`、`failed`），哪一步、系统的原话（`failed` 才有）。
    Failed {
        verb: Verb,
        code: String,
        step: String,
        said: String,
    },
}

/// 读命令印的那一行；读不懂的（核心旧、不认 `--format`）当没成，原话是它印的。
pub fn read(verb: Verb, stdout: &str) -> Reply {
    let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
    match serde_json::from_str::<Value>(stdout.trim()) {
        Ok(got) if got["installed"].is_boolean() => Reply::Done {
            verb,
            installed: got["installed"] == true,
            on: got["on"].as_bool(),
            file: text(&got["file"]),
        },
        Ok(got) if got["error"].is_string() => Reply::Failed {
            verb,
            code: text(&got["error"]),
            step: text(&got["step"]),
            said: text(&got["said"]),
        },
        _ => Reply::Failed {
            verb,
            code: "failed".to_string(),
            step: String::new(),
            said: stdout.trim().to_string(),
        },
    }
}

/// 跑这条命令的那一个程序：`MIYU_CORE_BIN`；没设的是 `None`。
pub fn program() -> Option<PathBuf> {
    std::env::var_os("MIYU_CORE_BIN")
        .filter(|bin| !bin.is_empty())
        .map(|bin| crate::local::expand_home(bin, std::env::home_dir().as_deref()))
}

/// 在跑的、跑完的。
pub struct Service {
    tx: Sender<Reply>,
    rx: Receiver<Reply>,
    /// 跑完叫醒主循环（`main.rs` 装上）。
    wake: Option<Wake>,
    /// 在跑着：跑着时要跑的排着，跑完再起（只记最后一次）。
    pub busy: bool,
    queued: Option<Verb>,
    /// 跑哪个程序：照 [`program`]，测试换成假的。
    program: Option<PathBuf>,
}

impl std::fmt::Debug for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Service")
            .field("busy", &self.busy)
            .finish_non_exhaustive()
    }
}

impl Default for Service {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            tx,
            rx,
            wake: None,
            busy: false,
            queued: None,
            program: None,
        }
    }
}

impl Service {
    /// 跑完怎么叫醒主循环（`main.rs` 装上）。
    pub fn set_wake(&mut self, wake: Wake) {
        self.wake = Some(wake);
    }

    /// 在后台跑一次；正跑着的排着，跑完再起。不知道核心程序在哪的不跑。
    pub fn run(&mut self, verb: Verb) {
        let Some(program) = self.program.clone().or_else(program) else {
            return;
        };
        if self.busy {
            self.queued = Some(verb);
            return;
        }
        self.busy = true;
        let (tx, wake) = (self.tx.clone(), self.wake.clone());
        std::thread::spawn(move || {
            let reply = match std::process::Command::new(&program)
                .args(["service", verb.word(), "--format", "json"])
                .stdin(std::process::Stdio::null())
                .output()
            {
                Ok(out) => read(verb, &String::from_utf8_lossy(&out.stdout)),
                Err(e) => Reply::Failed {
                    verb,
                    code: "failed".to_string(),
                    step: program.display().to_string(),
                    said: e.to_string(),
                },
            };
            // 界面已经关了的，跑完也没人收。
            if tx.send(reply).is_ok()
                && let Some(wake) = wake
            {
                wake();
            }
        });
    }

    /// 收下跑完的；排着的接着起。
    pub fn take(&mut self) -> Option<Reply> {
        let reply = self.rx.try_recv().ok()?;
        self.busy = false;
        if let Some(verb) = self.queued.take() {
            self.run(verb);
        }
        Some(reply)
    }
}

#[cfg(test)]
mod tests;
