//! 伪终端里的端到端测试：管理员模式（蓝图 `tui.md`「管理员模式」，2026-10-24 项目主人定）。真界面连一份真核心：配置页
//! `Shift+A` 进，开着时改的写系统配置（请求带 `scope`，核心才收）；退了以后写个人设置。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::Script;
use support::{Home, Tui};

/// 等文件里出现 `want`。
fn wait_file(home: &Home, tui: &mut Tui, file: &str, want: &str) {
    let path = home.root().join(file);
    let end = Instant::now() + support::WAIT;
    loop {
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        if text.contains(want) {
            return;
        }
        assert!(
            Instant::now() < end,
            "{file} 里没有「{want}」：\n{text}\n屏幕：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(100));
    }
}

/// 通用页里光标挪到「回复完成后折叠时间线」那一行。
fn to_fold(tui: &mut Tui) {
    for _ in 0..60 {
        let on = tui
            .lines()
            .iter()
            .any(|l| l.contains('┃') && l.contains("完成后折叠"));
        if on {
            return;
        }
        tui.key(b"j");
    }
    panic!("挪不到「完成后折叠」：\n{}", tui.lines().join("\n"));
}

#[test]
fn in_admin_mode_a_switch_goes_to_the_system_config_and_back_to_personal_after() {
    let home = Home::with_settings(Script::new([]), "");
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    tui.wait_for("显示");
    // 管理员（机主）：主菜单的按键提示多一段「A 管理员模式」。
    tui.wait_for("A 管理员模式");
    tui.key(b"A");
    tui.wait_for("· 管理员模式");
    tui.key(b"\r");
    tui.wait_for("界面语言");
    to_fold(&mut tui);
    tui.key(b"\r");
    // 开着：写进系统配置，个人设置里没有。
    wait_file(
        &home,
        &mut tui,
        "system/config.toml",
        "timeline_fold = false",
    );
    assert!(
        !home.settings().contains("timeline_fold"),
        "{}",
        home.settings()
    );
    // 退了：再切一次，写进个人设置。
    tui.key(b"A");
    let end = Instant::now() + support::WAIT;
    while tui.shows("· 管理员模式") {
        assert!(Instant::now() < end, "没退出：\n{}", tui.lines().join("\n"));
        tui.pump(Duration::from_millis(50));
    }
    tui.key(b"\r");
    wait_file(
        &home,
        &mut tui,
        "home/alice/settings.toml",
        "timeline_fold = true",
    );
}
