//! 伪终端里的端到端测试：关掉终端窗口（蓝图 `tui.md`「空闲不醒」第 4 条）。外面先忽略了挂断信号（例如套了 `nohup`）的，
//! 关窗以后也要退出，不在读挂断的终端上空转、一直占着一个核。关窗时界面要是静着（不往终端写），只有读终端这一路能发现，
//! 这里测的就是这一种。只有 Linux 上看挂断（`reader/hangup.rs`）。
#![cfg(target_os = "linux")]

mod support;

use std::time::Duration;

use miyu_session::testkit::Script;
use support::Home;

#[test]
fn closing_the_window_ends_it_even_when_hangups_are_ignored() {
    let home = Home::new(Script::new([]));
    let program = format!("trap '' HUP; exec '{}'", env!("CARGO_BIN_EXE_miyu-tui"));
    let mut tui = home.tui_via("zh_CN.UTF-8", "/bin/sh", &["-c", &program]);
    tui.wait_for("Tab 切换权限级别");
    // 等它静下来、不再往终端写（同 `tests/idle.rs`）：关窗以后还要写的，写失败就退了，测不到空转的那一种。
    tui.pump(Duration::from_secs(4));
    tui.hang_up();
    assert!(
        tui.gone_within(Duration::from_secs(5)),
        "关掉窗口 5 秒了界面还在（收不到挂断信号时没发现终端挂断）"
    );
}
