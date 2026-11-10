//! 伪终端里的端到端测试：系统通知照会话的人格来（蓝图 `tui.md`「系统通知」第 3 条，2026-11-10 项目主人）——标题写人格的名字，
//! 不写程序名；无人格的会话照旧写程序名。用假的 `notify-send` 记下参数，不真弹到桌面上。

#![cfg(unix)]

mod support;

use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 起一个界面：`persona` 是测具设的人格（`-` 是无人格），说一句、等回答完，交回假 `notify-send` 记下的参数（一行一个）。
fn notified(persona: &str) -> String {
    let home = Home::new(Script::new([Play::Says("好的。")]));
    home.persona("mei", "[persona]\nname = \"小美\"\n", None);
    let bin = home.root().join("notify-bin");
    std::fs::create_dir_all(&bin).expect("测试目录");
    let log = home.root().join("notify-args.log");
    std::fs::write(&log, "").expect("测试日志");
    let file = bin.join("notify-send");
    std::fs::write(
        &file,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"$NOTIFY_TEST_REPORTS\"\n",
    )
    .expect("假命令");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).expect("可执行");
    // 自己的假命令在最前面，接着测具的假通知、放音程序（提示音别真响），再是原来的 `PATH`。
    let path = format!(
        "{}:{}:{}",
        bin.display(),
        support::fake_notifiers(home.root()).display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let log_name = log.display().to_string();
    let mut tui = home.tui_with(
        "zh_CN.UTF-8",
        &[
            ("PATH", &path),
            ("NOTIFY_TEST_REPORTS", &log_name),
            ("MIYU_TUI_PERSONA", persona),
        ],
    );
    tui.wait_for("开放权限");
    // 终端报「离开」：不在前台才弹。
    tui.send(b"\x1b[O");
    tui.say("在吗");
    tui.wait_for("好的。");
    tui.pump(Duration::from_secs(2));
    std::fs::read_to_string(&log).expect("通知日志")
}

#[test]
#[cfg(not(target_os = "macos"))]
fn the_notification_title_is_the_personas_name() {
    let args = notified("mei");
    let title = args.lines().skip_while(|l| *l != "--").nth(1);
    assert_eq!(title, Some("小美"), "标题写人格的名字：\n{args}");
    assert!(
        !args.lines().any(|l| l == "-i"),
        "没头像的不带图标：\n{args}"
    );
}

#[test]
#[cfg(not(target_os = "macos"))]
fn a_session_without_a_persona_keeps_the_program_name() {
    let args = notified("-");
    let title = args.lines().skip_while(|l| *l != "--").nth(1);
    assert_eq!(title, Some("Miyu"), "无人格的照旧写程序名：\n{args}");
}
