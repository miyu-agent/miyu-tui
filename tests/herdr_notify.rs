//! herdr 通知：伪终端真 TUI、假核心和上报程序，不发真的桌面通知。
#![cfg(unix)]
mod support;
use miyu_session::testkit::{Play, Script};
use std::{
    os::unix::fs::PermissionsExt,
    time::{Duration, Instant},
};
use support::Home;

#[test]
fn a_reply_reports_to_herdr_and_sends_one_desktop_notification() {
    let home = Home::new(Script::new([Play::Says("通知验收回答。")]));
    let bin = home.root().join("notify-bin");
    std::fs::create_dir_all(&bin).expect("测试目录");
    let log = home.root().join("reports.log");
    std::fs::write(&log, "").expect("测试日志");
    for name in ["herdr-test", "notify-send", "osascript"] {
        let file = bin.join(name);
        std::fs::write(
            &file,
            format!("#!/bin/sh\nprintf '%s\\n' '{name}' \"$*\" >> \"$HERDR_TEST_REPORTS\"\n"),
        )
        .expect("假命令");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).expect("可执行");
    }
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let binary = bin.join("herdr-test").display().to_string();
    let log_name = log.display().to_string();
    let mut tui = home.tui_with(
        "zh_CN.UTF-8",
        &[
            ("PATH", &path),
            ("HERDR_ENV", "1"),
            ("HERDR_PANE_ID", "p-test"),
            ("HERDR_BIN_PATH", &binary),
            ("HERDR_TEST_REPORTS", &log_name),
        ],
    );
    tui.wait_for("工作区");
    tui.send(b"\x1b[O");
    tui.say("测试通知");
    tui.wait_for("通知验收回答。");
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        tui.pump(Duration::from_millis(50));
        let reported = std::fs::read_to_string(&log).expect("上报日志");
        if reported.matches("--state idle").count() >= 2
            && (reported.contains("notify-send") || reported.contains("osascript"))
        {
            assert!(reported.contains("--state working"));
            assert!(
                reported
                    .lines()
                    .filter(|line| *line == "notify-send" || *line == "osascript")
                    .count()
                    == 1,
                "{reported}"
            );
            break;
        }
        assert!(Instant::now() < until, "{reported}");
    }
}

/// herdr 报不上去时不往终端写字（review B7，2026-10-11）：上报在后台线程里，原来失败了 `eprintln!`，写进的是全屏界面
/// 所在的终端，界面只重画变了的格子，那行字一直留着。
#[test]
fn a_failing_herdr_report_writes_nothing_on_the_screen() {
    let home = Home::new(Script::new([Play::Says("回答完了。")]));
    let missing = home.root().join("no-such-herdr").display().to_string();
    let mut tui = home.tui_with(
        "zh_CN.UTF-8",
        &[
            ("HERDR_ENV", "1"),
            ("HERDR_PANE_ID", "p-test"),
            ("HERDR_BIN_PATH", &missing),
        ],
    );
    tui.wait_for("工作区");
    tui.say("你好");
    tui.wait_for("回答完了。");
    tui.pump(Duration::from_millis(500));
    assert!(
        !tui.lines().iter().any(|l| l.contains("herdr")),
        "{}",
        tui.lines().join("\n")
    );
}

/// 长期目标跑两轮（2026-10-17 项目主人报：每开一轮续轮都弹一次「已完成」）：第一轮做完、接着开续轮的不弹，目标做完
/// 那一轮才弹，一共一次（蓝图「系统通知」第 1 条）。
#[test]
fn a_goal_running_two_rounds_notifies_once_when_it_is_done() {
    let read = serde_json::json!({"path": "notes.md"}).to_string();
    let done = serde_json::json!({"action": "complete"}).to_string();
    let home = Home::with_tools(
        Script::new([
            Play::Calls(vec![("read".into(), read)]),
            Play::Says("第一轮看了笔记。"),
            Play::Calls(vec![("goal".into(), done)]),
            Play::Says("目标做完了。"),
        ]),
        "",
    );
    std::fs::write(home.work.join("notes.md"), "第一行\n").expect("笔记");
    let bin = home.root().join("notify-bin");
    std::fs::create_dir_all(&bin).expect("测试目录");
    let log = home.root().join("reports.log");
    std::fs::write(&log, "").expect("测试日志");
    for name in ["notify-send", "osascript"] {
        let file = bin.join(name);
        std::fs::write(
            &file,
            format!("#!/bin/sh\nprintf '%s\\n' '{name}' >> \"$NOTIFY_TEST_REPORTS\"\n"),
        )
        .expect("假命令");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).expect("可执行");
    }
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let log_name = log.display().to_string();
    let mut tui = home.tui_with(
        "zh_CN.UTF-8",
        &[("PATH", &path), ("NOTIFY_TEST_REPORTS", &log_name)],
    );
    tui.wait_for("工作区");
    // 终端报「离开」：不在前台才弹（第 1、2 条）。
    tui.send(b"\x1b[O");
    tui.say("/goal 看完笔记");
    tui.wait_for("目标做完了。");
    tui.pump(Duration::from_secs(2));
    let reported = std::fs::read_to_string(&log).expect("通知日志");
    assert_eq!(
        reported
            .lines()
            .filter(|l| *l == "notify-send" || *l == "osascript")
            .count(),
        1,
        "两轮只弹一次：\n{reported}\n{}",
        tui.lines().join("\n")
    );
}

/// 续轮开的一轮没调工具：核心记成等人开口，这时才弹（「已完成」欠到目标停下来，蓝图「系统通知」第 1 条）。
#[test]
fn a_goal_round_that_waits_for_you_notifies_once() {
    let home = Home::with_tools(Script::new([Play::Says("先问一句：笔记在哪？")]), "");
    std::fs::write(home.work.join("notes.md"), "第一行\n").expect("笔记");
    let bin = home.root().join("notify-bin");
    std::fs::create_dir_all(&bin).expect("测试目录");
    let log = home.root().join("reports.log");
    std::fs::write(&log, "").expect("测试日志");
    for name in ["notify-send", "osascript"] {
        let file = bin.join(name);
        std::fs::write(
            &file,
            format!("#!/bin/sh\nprintf '%s\\n' '{name}' >> \"$NOTIFY_TEST_REPORTS\"\n"),
        )
        .expect("假命令");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).expect("可执行");
    }
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let log_name = log.display().to_string();
    let mut tui = home.tui_with(
        "zh_CN.UTF-8",
        &[("PATH", &path), ("NOTIFY_TEST_REPORTS", &log_name)],
    );
    tui.wait_for("工作区");
    // 终端报「离开」：不在前台才弹（第 1、2 条）。
    tui.send(b"\x1b[O");
    tui.say("/goal 看完笔记");
    tui.wait_for("先问一句：笔记在哪？");
    tui.pump(Duration::from_secs(2));
    let reported = std::fs::read_to_string(&log).expect("通知日志");
    assert_eq!(
        reported
            .lines()
            .filter(|l| *l == "notify-send" || *l == "osascript")
            .count(),
        1,
        "等人开口时弹一次：\n{reported}\n{}",
        tui.lines().join("\n")
    );
}
