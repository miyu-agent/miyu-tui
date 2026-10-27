//! 空闲不醒（蓝图 `tui.md`「空闲不醒」、「空会话的首页」第 8 条）：吉祥物醒着的那一阵过了、停住以后，界面不再往终端
//! 写一个字节；Linux 上再数界面所有线程被叫醒几次，一次都不许有（读按键的线程原来每 50 毫秒醒一次看要不要停）。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 吉祥物醒着的那一阵、哈欠都调短（吉祥物包能改的那几格，本机试画用 `MIYU_TUI_MASCOT`），测试不用等十几秒。
const SHORT: &str = r#"{"idle": {"awake_ms": 300}, "mouth": {"yawn_ms": 200}}"#;

/// 停住以后量多久。
const QUIET: Duration = Duration::from_secs(2);

/// 写一份调短的吉祥物模型，交回它的路径。
fn short_mascot(home: &Home) -> String {
    let path = home.work.join("mascot.json");
    std::fs::write(&path, SHORT).expect("写模型文件");
    path.display().to_string()
}

/// 吉祥物出厂关着（2026-10-26 项目主人：太吃资源），这里测的就是它：首页、侧边栏都打开。
const MASCOT_ON: &str = "[tui]\nmascot_home = true\nmascot_sidebar = true\n";

/// 等醒着的那一阵、哈欠、转回正前方都过去，再量 [`QUIET`] 这么久：一个字节都不写，线程一次都不醒。
fn assert_quiet(tui: &mut Tui, what: &str) {
    tui.pump(Duration::from_secs(3));
    let before = tui.wakeups();
    tui.record();
    tui.pump(QUIET);
    let written = tui.recorded();
    let after = tui.wakeups();
    assert!(
        written.is_empty(),
        "{what}：停住以后还写了 {} 个字节：{:?}",
        written.len(),
        String::from_utf8_lossy(&written)
    );
    if let (Some(before), Some(after)) = (before, after) {
        assert_eq!(after - before, 0, "{what}：停住以后界面的线程还醒了");
    }
}

#[test]
fn the_home_screen_settles_and_stays_silent_until_something_happens() {
    let home = Home::with_settings(Script::new([]), MASCOT_ON);
    let mascot = short_mascot(&home);
    let mut tui = home.tui_with("zh_CN.UTF-8", &[("MIYU_TUI_MASCOT", mascot.as_str())]);
    tui.wait_for("Tab 切换权限级别");
    assert!(
        tui.shows("@"),
        "首页画着吉祥物：\n{}",
        tui.lines().join("\n")
    );
    assert_quiet(&mut tui, "首页");
    // 打一个字：醒过来动一阵，又停住。
    tui.record();
    tui.type_text("在");
    assert!(!tui.recorded().is_empty(), "打字照常画");
    assert_quiet(&mut tui, "打过字的首页");
}

#[test]
fn a_finished_turn_with_the_sidebar_mascot_stays_silent() {
    let home = Home::with_settings(Script::new([Play::Says("好。")]), MASCOT_ON);
    let mascot = short_mascot(&home);
    let mut tui = home.tui_wide_with("zh_CN.UTF-8", 130, &[("MIYU_TUI_MASCOT", mascot.as_str())]);
    tui.wait_for("Tab 切换权限级别");
    tui.say("在吗");
    tui.wait_for("好。");
    assert!(
        tui.shows("@"),
        "侧边栏画着吉祥物：\n{}",
        tui.lines().join("\n")
    );
    assert_quiet(&mut tui, "会话里");
}
