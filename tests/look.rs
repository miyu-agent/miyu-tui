//! 主题（蓝图 `tui.md`「主题」第 3、5、6 条）：伪终端里的真界面连一份真核心；纯 ASCII 编进了程序（todolist 63），别的主题包照用户装的那样放在 alice 的家里。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::Script;
use support::{Home, Tui};

/// 等到屏幕上没有圆角、有 ASCII 的框（输入框的角是 `+`），提示符也换了（`layout.json` 上盖的那一节）。
fn wait_ascii(tui: &mut Tui) {
    let end = Instant::now() + support::WAIT;
    loop {
        let lines = tui.lines();
        let rounded = lines
            .iter()
            .any(|l| l.contains('╭') || l.contains('╰') || l.contains('❯'));
        let ascii = lines.iter().any(|l| l.trim_start().starts_with("+--"));
        if !rounded && ascii {
            return;
        }
        assert!(
            Instant::now() < end,
            "没换成纯 ASCII：\n{}",
            lines.join("\n")
        );
        tui.pump(Duration::from_millis(100));
    }
}

#[test]
fn a_chosen_theme_package_redraws_the_frames() {
    // 配置里选了哪个就照哪个画；纯 ASCII 编进了程序（todolist 63，原来是软件包），不用装。
    let home = Home::with_settings(Script::new([]), "[tui]\nlook = \"ascii\"\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    wait_ascii(&mut tui);
    // 选择窗：第一行「默认主题」，第二行内置的「纯 ASCII」，再下面是装了的主题包，默认主题包不另列。
    tui.say("/config");
    tui.open_menu("外观");
    tui.wait_for("纯 ASCII");
    tui.key(b"\r");
    tui.wait_for("默认主题");
    let rows = tui
        .lines()
        .iter()
        .filter(|l| l.contains("默认主题"))
        .count();
    assert_eq!(rows, 1, "默认主题包不另列：\n{}", tui.lines().join("\n"));
    // 选择窗停在现在的值上：往上一行是默认主题，选它；回到对话，圆角回来。
    tui.key(b"k");
    tui.key(b"\r");
    tui.wait_for("外观");
    tui.key(b"\x1b");
    tui.pump(Duration::from_millis(300));
    tui.key(b"\x1b");
    tui.wait_for("╭");
}

#[test]
fn a_missing_theme_package_falls_back_to_the_default_and_says_so() {
    let home = Home::with_settings(Script::new([]), "[tui]\nlook = \"nope\"\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.wait_for("主题包无法读取，已使用默认主题");
    assert!(tui.shows("╭"), "照默认主题画：\n{}", tui.lines().join("\n"));
}

#[test]
fn the_second_start_draws_the_chosen_theme_from_its_first_frame() {
    // 2026-10-21 项目主人：选了主题包的，起来前两帧照默认主题画、再跳成纯 ASCII。记下上次用的，第一帧就照它。
    let home = Home::with_settings(Script::new([]), "[tui]\nlook = \"ascii\"\n");
    {
        let mut first = home.tui("zh_CN.UTF-8");
        first.wait_for("开放权限");
        wait_ascii(&mut first);
        first.pump(Duration::from_millis(300));
    }
    let mut tui = home.tui("zh_CN.UTF-8");
    let start = tui.record_from_here();
    tui.wait_for("开放权限");
    tui.pump(Duration::from_millis(500));
    let recorded = tui.recorded();
    for frame in support::frames(start, &recorded) {
        assert!(
            !frame.iter().any(|l| l.contains('╭') || l.contains('❯')),
            "有一帧照默认主题画了：\n{}",
            frame.join("\n")
        );
    }
}

/// todolist 63（2026-11-13 项目主人）：Linux 控制台（`TERM=linux`）里没选过主题的，照内置的纯 ASCII 画，界面是中文的提示一次
/// 控制台显示不了中文；写过主题的（哪怕写的是默认主题）照写的。
#[test]
fn the_linux_console_draws_plain_ascii_unless_a_theme_was_chosen() {
    let home = Home::new(Script::new([]));
    let mut tui = home.tui_with("zh_CN.UTF-8", &[("TERM", "linux"), ("COLORTERM", "")]);
    tui.wait_for("开放权限");
    wait_ascii(&mut tui);
    tui.wait_for("Linux 控制台显示不了中文");
    drop(tui);
    let chosen = Home::with_settings(Script::new([]), "[tui]\nlook = \"tui-theme-default\"\n");
    let mut tui = chosen.tui_with("zh_CN.UTF-8", &[("TERM", "linux"), ("COLORTERM", "")]);
    tui.wait_for("开放权限");
    tui.pump(Duration::from_millis(800));
    assert!(
        tui.shows("╭"),
        "写了默认主题的照默认主题画：\n{}",
        tui.lines().join("\n")
    );
}
