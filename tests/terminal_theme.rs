//! 伪终端里的端到端测试：跟着终端的颜色（蓝图 `tui.md`「终端配色」「主题」第 11 条，2026-10-15 项目主人定，设计 13 的
//! H15）。终端配色照起来时问到的终端颜色生成真彩色；窗口拿回焦点时再问一次，变了照新的重挑；闲着不去问。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::Script;
use support::{Home, Tui};
use vt100::Color;

/// 深底浅字、浅底深字。
const DARK: ((u8, u8, u8), (u8, u8, u8)) = ((200, 200, 200), (20, 20, 20));
const LIGHT: ((u8, u8, u8), (u8, u8, u8)) = ((30, 30, 30), (235, 235, 235));

/// 16 色都是灰，4 号（蓝）是 `blue`。
fn sixteen(blue: (u8, u8, u8)) -> [(u8, u8, u8); 16] {
    let mut colors = [(128, 128, 128); 16];
    colors[4] = blue;
    colors
}

/// 左下角那个「工作区」的字色（权限级别那一格，主题的 `workspace`）。
fn level_color(tui: &Tui) -> Option<Color> {
    let lines = tui.lines();
    let row = lines.iter().rposition(|l| l.contains("工作区"))?;
    let col = lines[row].split("工作区").next()?.chars().count();
    Some(tui.foreground(u16::try_from(row).ok()?, u16::try_from(col).ok()?))
}

/// 等左下角那个「工作区」变成 `want` 的颜色，最多 10 秒。
fn until_level(tui: &mut Tui, want: Color, what: &str) {
    let end = Instant::now() + Duration::from_secs(10);
    while level_color(tui) != Some(want) {
        assert!(
            Instant::now() < end,
            "{what}：{:?}\n{}",
            level_color(tui),
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(100));
    }
}

/// 拿回焦点（`CSI I`，焦点报告开着）。
fn regain_focus(tui: &mut Tui) {
    tui.send(b"\x1b[O");
    tui.pump(Duration::from_millis(100));
    tui.send(b"\x1b[I");
}

#[test]
fn terminal_colors_take_the_answered_bytes_and_follow_them_after_regaining_focus() {
    let home = Home::with_settings(Script::new([]), "[tui]\ntheme = \"terminal\"\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.answer_ground(DARK.0, DARK.1);
    tui.answer_colors(sixteen((0x40, 0x80, 0xff)));
    tui.wait_for("工作区");
    // 主题文件写的是 `blue`：真彩色、问到了，画成终端答的那个蓝的字节。
    until_level(&mut tui, Color::Rgb(0x40, 0x80, 0xff), "起来时问到的 4 号");
    // 闲着不去问。
    tui.record();
    tui.pump(Duration::from_millis(1500));
    let idle = tui.recorded();
    assert!(
        !idle.windows(4).any(|w| w == b"\x1b]4;"),
        "闲着又问了终端的颜色"
    );
    // 终端换了配色：拿回焦点时再问，照新的画。
    tui.answer_colors(sixteen((0xff, 0x80, 0x40)));
    regain_focus(&mut tui);
    until_level(
        &mut tui,
        Color::Rgb(0xff, 0x80, 0x40),
        "拿回焦点再问到的 4 号",
    );
}

#[test]
fn the_automatic_mode_turns_light_when_the_terminal_does_and_focus_comes_back() {
    // 明暗「自动」：起来时深底用 tokyonight 深色那一份；终端换成浅底，拿回焦点再问，换浅色那一份。
    let home = Home::with_settings(Script::new([]), "");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.answer_ground(DARK.0, DARK.1);
    tui.wait_for("工作区");
    until_level(&mut tui, Color::Rgb(0x7a, 0xa2, 0xf7), "深色那一份");
    tui.answer_ground(LIGHT.0, LIGHT.1);
    regain_focus(&mut tui);
    until_level(&mut tui, Color::Rgb(0x2e, 0x7d, 0xe9), "换成浅色那一份");
}
