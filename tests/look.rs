//! 主题包（蓝图 `tui.md`「主题」第 3、5、6 条）：伪终端里的真界面连一份真核心，主题包照用户装的那样放在 alice 的家里。

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
    // 2026-10-21 项目主人：默认主题、纯 ASCII 样板都是软件包；配置里选了哪个就照哪个画。
    let home = Home::with_settings(Script::new([]), "[tui]\nlook = \"miyu-tui-theme-ascii\"\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    wait_ascii(&mut tui);
    // 选择窗：第一行「默认主题」，下面是装了的主题包，默认主题包不另列。
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
    tui.wait_for("工作区");
    tui.wait_for("主题包无法读取，已使用默认主题");
    assert!(tui.shows("╭"), "照默认主题画：\n{}", tui.lines().join("\n"));
}

#[test]
fn the_second_start_draws_the_chosen_theme_from_its_first_frame() {
    // 2026-10-21 项目主人：选了主题包的，起来前两帧照默认主题画、再跳成纯 ASCII。记下上次用的，第一帧就照它。
    let home = Home::with_settings(Script::new([]), "[tui]\nlook = \"miyu-tui-theme-ascii\"\n");
    {
        let mut first = home.tui("zh_CN.UTF-8");
        first.wait_for("工作区");
        wait_ascii(&mut first);
        first.pump(Duration::from_millis(300));
    }
    let mut tui = home.tui("zh_CN.UTF-8");
    let start = tui.record_from_here();
    tui.wait_for("工作区");
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
