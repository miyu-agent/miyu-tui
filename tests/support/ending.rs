//! 从外面结束界面（`tests/quit_signals.rs`）：给它发信号、看它是怎么结束的、终端还原了没有。

use std::time::{Duration, Instant};

use super::Tui;

impl Tui {
    /// 给界面发 `signal`。
    pub fn signal(&self, signal: rustix::process::Signal) {
        let pid = self.pid().and_then(|p| i32::try_from(p).ok());
        let pid = pid
            .and_then(rustix::process::Pid::from_raw)
            .expect("界面有进程号");
        rustix::process::kill_process(pid, signal).expect("发得出信号");
    }

    /// 等界面结束（最多 `wait`，一边接着读它写的）：交回是不是被信号结束的（信号的名字），没结束的是 `None`。
    pub fn ended_within(&mut self, wait: Duration) -> Option<Option<String>> {
        let end = Instant::now() + wait;
        while Instant::now() < end {
            if let Ok(Some(status)) = self.child.try_wait() {
                return Some(status.signal().map(str::to_string));
            }
            self.pump(Duration::from_millis(50));
        }
        None
    }

    /// 终端这会儿照常回显、按行读（`ICANON`、`ECHO` 都开着）：原始模式还原了。只有 Linux 上看（测试的 `rustix` 带 `termios`）。
    #[cfg(target_os = "linux")]
    pub fn cooked(&self) -> bool {
        use rustix::fs::{Mode, OFlags, open};
        use rustix::termios::{LocalModes, tcgetattr};
        let path = self.master.tty_name().expect("伪终端有名字");
        let tty = open(&path, OFlags::RDWR | OFlags::NOCTTY, Mode::empty()).expect("开得了伪终端");
        let modes = tcgetattr(&tty).expect("读得了终端设置").local_modes;
        modes.contains(LocalModes::ICANON | LocalModes::ECHO)
    }
}
