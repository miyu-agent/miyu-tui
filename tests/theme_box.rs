//! `/theme` 的配色框（蓝图 `tui.md`「主题」第 6 条，2026-10-15 项目主人：出列表选，选好写进设置）：开框列几种配色、打开
//! 时用着的那种标「当前」；挪到哪一种当场换样子；`Esc` 换回原来的、不写设置；`Enter` 定下写进个人设置的 `tui.theme`。

mod support;

use std::time::Duration;

use miyu_session::testkit::Script;
use support::Home;
use vt100::Color;

/// tokyonight 深色那一套铺的底色。
const NIGHT: Color = Color::Rgb(0x1a, 0x1b, 0x26);

#[test]
fn the_theme_command_opens_a_list_previews_and_writes_the_choice() {
    let home = Home::with_settings(Script::new([]), "[tui]\ntheme_mode = \"dark\"\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.pump(Duration::from_millis(800));
    assert_eq!(tui.background(0, 0), NIGHT, "起来是 tokyonight 深色");
    tui.say("/theme");
    // 命令列表那一行的说明里也有这几种的名字：等框里的「当前」。
    tui.wait_for("当前");
    let current = tui
        .lines()
        .into_iter()
        .find(|l| l.contains("Tokyo Night"))
        .unwrap_or_default();
    assert!(
        current.contains("当前"),
        "用着的那种标当前：\n{}",
        tui.lines().join("\n")
    );
    // 往下挪到终端配色：当场换样子（终端配色不铺底色）。
    tui.key(b"\x1b[B");
    tui.pump(Duration::from_millis(300));
    assert_ne!(tui.background(0, 0), NIGHT, "挪到哪当场换成哪");
    // Esc：换回原来的，什么都不写。
    tui.key(b"\x1b");
    tui.pump(Duration::from_millis(300));
    assert_eq!(tui.background(0, 0), NIGHT, "Esc 换回原来的");
    assert!(!tui.shows("当前"), "框关了");
    assert!(
        !home.settings().contains("theme = "),
        "没定下不写：{}",
        home.settings()
    );
    // 再开，挪到终端配色按 Enter：写进个人设置，框关了，样子留着。
    tui.say("/theme");
    tui.wait_for("当前");
    tui.key(b"\x1b[B");
    tui.key(b"\r");
    tui.pump(Duration::from_millis(800));
    assert!(
        home.settings().contains("theme = \"terminal\""),
        "定下的写进个人设置：{}",
        home.settings()
    );
    assert!(!tui.shows("当前"), "框关了");
    assert_ne!(tui.background(0, 0), NIGHT, "样子留着");
}
