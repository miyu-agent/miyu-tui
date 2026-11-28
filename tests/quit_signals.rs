//! 伪终端里的端到端测试：从外面结束界面（2026-11-28 审计第 4 条）。`kill`、`pkill`、`timeout` 发的 `SIGTERM`、`SIGINT`：
//! 终端照样还原（离开备用屏、关鼠标上报和括号粘贴、关 kitty 键盘协议、亮光标、退出原始模式），再照信号结束。等图形编辑器
//! （`code --wait`）时在终端按 `Ctrl+C`：交给编辑器办，界面不退。
#![cfg(unix)]

mod support;

use std::time::Duration;

use miyu_session::testkit::Script;
use rustix::process::Signal;
use support::Home;

/// 结束时终端最后收到的、还原用的几段。
const RESTORED: [(&str, &[u8]); 5] = [
    ("离开备用屏", b"\x1b[?1049l"),
    ("关鼠标上报", b"\x1b[?1000l"),
    ("关括号粘贴", b"\x1b[?2004l"),
    ("关 kitty 键盘协议", b"\x1b[<1u"),
    ("亮光标", b"\x1b[?25h"),
];

/// 起界面，等第一帧，发 `signal`：照信号结束，结束以前把终端还原了。
fn ends_cleanly_on(signal: Signal, name: &str) {
    let home = Home::new(Script::new([]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.pump(Duration::from_millis(300));
    tui.record();
    tui.signal(signal);
    let ended = tui.ended_within(Duration::from_secs(10));
    tui.pump(Duration::from_millis(300));
    let out = tui.recorded();
    let ended = ended.expect("收到信号 10 秒了还没结束");
    for (what, bytes) in RESTORED {
        assert!(
            out.windows(bytes.len()).any(|w| w == bytes),
            "收到 {name} 结束时没有{what}：{:?}",
            String::from_utf8_lossy(&out)
        );
    }
    #[cfg(target_os = "linux")]
    assert!(tui.cooked(), "收到 {name} 结束以后终端还在原始模式里");
    assert!(ended.is_some(), "照 {name} 的惯例被信号结束，不是自己退出");
}

#[test]
fn a_terminate_signal_puts_the_terminal_back() {
    ends_cleanly_on(Signal::TERM, "SIGTERM");
}

#[test]
fn an_interrupt_signal_puts_the_terminal_back() {
    ends_cleanly_on(Signal::INT, "SIGINT");
}

#[test]
fn ctrl_c_while_waiting_for_the_editor_leaves_the_interface_running() {
    // 等编辑器退出的这段时间终端是熟模式，`Ctrl+C` 发给整个前台进程组。假编辑器只睡着，像 `code --wait` 等窗口关掉。
    let home = Home::new(Script::new([]));
    let script = home.work.join("slow-editor.sh");
    std::fs::write(&script, "#!/bin/sh\nsleep 30\n").expect("写得了假编辑器");
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&script, mode).expect("改得了权限");
    }
    let editor = script.display().to_string();
    let mut tui = home.tui_with("zh_CN.UTF-8", &[("VISUAL", &editor), ("EDITOR", &editor)]);
    tui.wait_for("开放权限");
    tui.type_text("原来的话");
    tui.pump(Duration::from_millis(300));
    tui.key(b"\x07");
    tui.pump(Duration::from_millis(800));
    tui.send(b"\x03");
    assert_eq!(
        tui.ended_within(Duration::from_secs(3)),
        None,
        "等编辑器时按 Ctrl+C 把界面结束了"
    );
    // 编辑器照 Ctrl+C 退了：回到输入框，原来的话还在。
    tui.wait_for("原来的话");
}
