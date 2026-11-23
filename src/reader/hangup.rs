//! 终端挂断了就收工（关掉终端窗口）。平常关窗时系统发挂断信号（SIGHUP），照默认就退出了；收不到的（外面先忽略了它，
//! 例如套了 `nohup`）只剩读终端这一路能发现，可 crossterm 读挂断的终端每次当场读到 0 字节（或者出错）就原地再读，不停
//! 也不报错，读按键那一路占满一个核、程序一直不退（LiteMiyu 实测到的，这里一样）。
//!
//! 所以另起一条线程，用不读的 `poll` 只等这个终端挂断（HUP、ERR、NVAL），平常一直睡着、不定时醒（蓝图「空闲不醒」）。
//! 等到了照挂断信号的默认做法退出：终端已经没了，不收拾屏幕。
//!
//! 只在 Linux 上看：macOS 的 `poll` 不认终端这种设备（手册 BUGS 一节），一上来就报 NVAL，会把好好的程序当成挂断；
//! Windows 关窗口时系统直接结束程序。

use std::fs::File;
use std::io::IsTerminal;
use std::os::fd::{AsFd, OwnedFd};
use std::sync::Once;
use std::thread;

use rustix::event::{PollFd, PollFlags, poll};

/// 挂断信号默认结束程序时 shell 看到的退出码（128 + SIGHUP）。
const HUNG_UP: i32 = 129;

/// 起看挂断的线程；一个进程只起一条。看不了（没有终端）的不起。
pub fn watch() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let Some(terminal) = terminal() else {
            return;
        };
        drop(
            thread::Builder::new()
                .name("hangup".into())
                .spawn(move || wait(&terminal)),
        );
    });
}

/// crossterm 读的是哪一个终端，这里看同一个：标准输入是终端的看它，不是的看 `/dev/tty`。
fn terminal() -> Option<OwnedFd> {
    if std::io::stdin().is_terminal() {
        return std::io::stdin().as_fd().try_clone_to_owned().ok();
    }
    File::open("/dev/tty").ok().map(OwnedFd::from)
}

/// 等到挂断就退出；等出别的（不该有）就不看了，不空转。
fn wait(terminal: &OwnedFd) {
    let mut fds = [PollFd::new(terminal, PollFlags::empty())];
    loop {
        match poll(&mut fds, None) {
            Ok(_) => {
                let got = fds[0].revents();
                if got.intersects(PollFlags::HUP | PollFlags::ERR | PollFlags::NVAL) {
                    std::process::exit(HUNG_UP);
                }
                if !got.is_empty() {
                    return;
                }
            }
            Err(rustix::io::Errno::INTR) => {}
            Err(_) => return,
        }
    }
}
