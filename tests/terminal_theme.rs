//! 伪终端里的端到端测试：跟着终端的颜色（蓝图 `tui.md`「终端配色」「主题」第 11 条，2026-10-15 项目主人定，设计 13 的
//! H15）。终端配色照起来时问到的终端颜色生成真彩色（2026-10-19 起照终端 16 色里的主色生成 MD3）；窗口拿回焦点时再问一次，
//! 变了照新的重挑；闲着不去问。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
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

/// 照主色走的那个颜色：运行状态行第一个字（流光照配色方案原来工作区那一档，`workspace`，跟着主色；LV-3 去掉工作区一级
/// 以前看的是左下角那个「工作区」）；没在回答的照左下角「开放权限」（主题的 `full`）。
fn level_color(tui: &Tui) -> Option<Color> {
    let lines = tui.lines();
    if let Some(top) = lines.iter().position(|l| l.trim_start().starts_with('╭'))
        && let Some(row) = top.checked_sub(2)
        && lines[row].trim_end().ends_with('s')
        && let Some(col) = lines[row].chars().position(|c| !c.is_whitespace())
    {
        return Some(tui.foreground(u16::try_from(row).ok()?, u16::try_from(col).ok()?));
    }
    let row = lines.iter().rposition(|l| l.contains("开放权限"))?;
    let col = lines[row].split("开放权限").next()?.chars().count();
    Some(tui.foreground(u16::try_from(row).ok()?, u16::try_from(col).ok()?))
}

/// 等 [`level_color`] 合 `want`，最多 10 秒。
fn until_level(tui: &mut Tui, want: impl Fn(Color) -> bool, what: &str) {
    let end = Instant::now() + Duration::from_secs(10);
    while !level_color(tui).is_some_and(&want) {
        assert!(
            Instant::now() < end,
            "{what}：{:?}\n{}",
            level_color(tui),
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(100));
    }
}

/// 偏蓝的真颜色。
fn bluish(color: Color) -> bool {
    matches!(color, Color::Rgb(r, _, b) if u16::from(b) > u16::from(r) + 40)
}

/// 偏橙的真颜色。
fn orange(color: Color) -> bool {
    matches!(color, Color::Rgb(r, _, b) if u16::from(r) > u16::from(b) + 40)
}

/// 拿回焦点（`CSI I`，焦点报告开着）。
fn regain_focus(tui: &mut Tui) {
    tui.send(b"\x1b[O");
    tui.pump(Duration::from_millis(100));
    tui.send(b"\x1b[I");
}

#[test]
fn terminal_colors_take_the_answered_bytes_and_follow_them_after_regaining_focus() {
    let home = Home::with_settings(Script::new([Play::Holds]), "[tui]\ntheme = \"terminal\"\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.answer_ground(DARK.0, DARK.1);
    tui.answer_colors(sixteen((0x40, 0x80, 0xff)));
    tui.wait_for("开放权限");
    // 开一轮、停着：运行状态行的流光照主色走（看它）。
    tui.say("在吗");
    tui.wait_for_line("…");
    // 真彩色、问到了：16 色里只有 4 号带颜色，照它生成 MD3，强调色是蓝的（不再是终端那一号的原字节）。
    until_level(&mut tui, bluish, "照起来时问到的 4 号生成");
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
    until_level(&mut tui, orange, "照拿回焦点再问到的 4 号生成");
}

#[test]
fn the_automatic_mode_turns_light_when_the_terminal_does_and_focus_comes_back() {
    // 明暗「自动」：起来时深底用 tokyonight 深色那一份；终端换成浅底，拿回焦点再问，换浅色那一份。
    let home = Home::with_settings(Script::new([]), "");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.answer_ground(DARK.0, DARK.1);
    tui.wait_for("开放权限");
    // 没在回答：看左下角「开放权限」（tokyonight 的 `full`，2026-11-10 起和原来工作区那一档同色）。
    until_level(
        &mut tui,
        |c| c == Color::Rgb(0x7a, 0xa2, 0xf7),
        "深色那一份",
    );
    tui.answer_ground(LIGHT.0, LIGHT.1);
    regain_focus(&mut tui);
    until_level(
        &mut tui,
        |c| c == Color::Rgb(0x2e, 0x7d, 0xe9),
        "换成浅色那一份",
    );
}

#[test]
fn typing_right_after_regaining_focus_keeps_every_character() {
    // 2026-11-17 实测：niri 代理先往别的窗口打过字、再给界面打一句中英混排的，焦点回来那一下再问终端颜色，读回话时把排在
    // 前面的按键一起读走了，输入框只剩前半截（到「spec.edi」）。终端在这里慢一拍才答：问句出去以后先到一串字，再到回话。
    let home = Home::with_settings(Script::new([]), "");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.answer_ground(DARK.0, DARK.1);
    tui.answer_colors(sixteen((0x40, 0x80, 0xff)));
    tui.wait_for("开放权限");
    tui.send(b"\x1b[O");
    tui.pump(Duration::from_millis(100));
    tui.hold_answers();
    tui.record();
    tui.send(b"\x1b[I");
    let end = Instant::now() + Duration::from_secs(5);
    let mut out = Vec::new();
    while !out.windows(5).any(|w| w == b"\x1b]10;") {
        assert!(Instant::now() < end, "拿回焦点没再问终端的颜色");
        tui.pump(Duration::from_millis(20));
        out.extend(tui.recorded());
        tui.record();
    }
    let typed = "联网搜索：EditorConfig 规范（spec.editorconfig.org）的版本号？";
    tui.send(typed.as_bytes());
    tui.pump(Duration::from_millis(100));
    tui.release_answers();
    let end = Instant::now() + Duration::from_secs(3);
    while tui.input() != typed && Instant::now() < end {
        tui.pump(Duration::from_millis(50));
    }
    assert_eq!(tui.input(), typed, "拿回焦点时打的字丢了");
}
