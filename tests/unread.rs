//! 伪终端里的端到端测试：会话读不了的两种（核心 YC-6、YC-2；蓝图 `tui.md`「连核心」第 7 条下）。
//!
//! - 日志中间坏了一行：重连以后订阅被拒 `session_read_only`，正文末尾写坏在第几行，输入框写怎么新开会话；还是要说的，核心拒，
//!   字退回框里、弹「只能看」。
//! - 一时读不了（日志读不出来）：重连以后订阅被拒 `session_unavailable`，弹「几秒后自动重试」；读得了以后到点再订阅，照常接着聊。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 开着的那一个会话的第一段日志。
fn log_of(home: &Home) -> std::path::PathBuf {
    let sessions = home.data().sessions_dir();
    let dir = std::fs::read_dir(&sessions)
        .expect("有会话目录")
        .filter_map(Result::ok)
        .find(|e| e.path().is_dir())
        .expect("开了一个会话")
        .path();
    dir.join("000000000001.jsonl")
}

#[test]
fn a_log_broken_in_the_middle_leaves_the_session_view_only() {
    let home = Home::new(Script::new([Play::Says("一。"), Play::Says("二。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("第一句");
    tui.wait_for_done();
    tui.say("第二句");
    tui.wait_for("二。");
    tui.wait_for_done();
    // 日志中间换掉一行（第一行留着：坏在第一行的是 `session_broken`），重启核心，界面重连、重新订阅
    let log = log_of(&home);
    let text = std::fs::read_to_string(&log).expect("读得了日志");
    let mut lines: Vec<&str> = text.lines().collect();
    let broken = lines.len() / 2;
    assert!(broken >= 1, "日志有好几行：{}", lines.len());
    lines[broken] = "{坏了";
    std::fs::write(&log, lines.join("\n") + "\n").expect("写得进去");
    home.stop_core();
    home.restart_core();
    tui.wait_for(&format!("会话日志第 {} 行坏了", broken + 1));
    tui.wait_for("这个会话只能看，输入 /new 新开会话");
    assert!(tui.shows("一。"), "坏行以前的照旧画着");
    tui.type_text("再说一句");
    tui.key(b"\r");
    tui.wait_for("这个会话只能看，不能接着聊");
    assert!(tui.shows("再说一句"), "{}", tui.lines().join("\n"));
}

/// 日志读不出来要靠权限挡住：Windows 上没有这一套，超级用户挡不住，都跳过。
#[cfg(unix)]
#[test]
fn a_session_unreadable_for_now_is_tried_again_and_carries_on() {
    use std::os::unix::fs::PermissionsExt;

    let home = Home::new(Script::new([Play::Says("一。"), Play::Says("接上了。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("第一句");
    tui.wait_for_done();
    // 会话目录读不了（同核心的测试 `session_unavailable.rs`）
    let log = log_of(&home);
    let dir = log.parent().expect("在会话目录里").to_path_buf();
    let was = std::fs::metadata(&dir).expect("有会话目录").permissions();
    home.stop_core();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o000)).expect("改得了权限");
    if std::fs::read(&log).is_ok() {
        eprintln!("挡不住读（超级用户），跳过");
        std::fs::set_permissions(&dir, was).expect("改回去");
        return;
    }
    home.restart_core();
    tui.wait_for("会话暂时无法读取，5 秒后自动重试");
    assert!(tui.shows("一。"), "不当成会话坏了：以前的照旧画着");
    std::fs::set_permissions(&dir, was).expect("改回去");
    // 到点再订阅成了：照常接着聊
    tui.say("还在吗");
    tui.wait_for("接上了。");
}
