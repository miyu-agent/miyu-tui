//! 伪终端里的端到端测试：运行状态行的流光一直照配色方案原来工作区那一档的颜色（蓝图 `tui.md`「运行状态行和排队的消息」
//! 第 1 条；2026-10-17 项目主人：开放权限时整行跟着变红不对，所有主题都要查）。LV-3 去掉了工作区一级，底栏不再有那个颜色可
//! 比，照配色方案写的比。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};
use vt100::Color;

/// 配色、明暗和那一套的工作区色（`resources/themes/*.json` 的 `workspace`）：出厂的几种配色各取深、浅。跟随壁纸和跟随头像是
/// 同一套算法，照一个主色生成（`tests/theme.rs` 验过同一个主色铺一样的底），这里用跟随头像代表，主色是绿的：工作区色跟着
/// 主色是绿。终端配色没问到终端颜色时照调色板 4 号（`blue`）原样画。2026-11-10 起开放权限也是这一档的颜色（项目主人：
/// 从红改成原来工作区的颜色），底栏的「开放权限」和流光都照它，离原来开放权限的红远。
const THEMES: [(&str, &str, Color); 6] = [
    ("tokyonight", "dark", Color::Rgb(0x7a, 0xa2, 0xf7)),
    ("tokyonight", "light", Color::Rgb(0x2e, 0x7d, 0xe9)),
    ("terminal", "dark", Color::Idx(4)),
    ("terminal", "light", Color::Idx(4)),
    ("persona", "dark", Color::Rgb(0x2f, 0xab, 0x34)),
    ("persona", "light", Color::Rgb(0x2f, 0xab, 0x34)),
];

/// 原来开放权限的红（tokyonight 深色的 `full`）：工作区那一档的颜色离它远。
const RED: Color = Color::Rgb(0xf7, 0x76, 0x8e);

/// 底栏左下角写着 `word` 的那一格的字色。
fn footer_color(tui: &Tui, word: &str) -> Option<Color> {
    let lines = tui.lines();
    let row = lines.iter().rposition(|l| l.contains(word))?;
    let col = lines[row].split(word).next()?.chars().count();
    Some(tui.foreground(u16::try_from(row).ok()?, u16::try_from(col).ok()?))
}

/// 运行状态行第一个字的字色：输入框上边框往上隔一行空的那一行。
fn pulse_color(tui: &Tui) -> Option<Color> {
    let lines = tui.lines();
    let top = lines.iter().position(|l| l.trim_start().starts_with('╭'))?;
    let row = top.checked_sub(2)?;
    let col = lines[row].chars().position(|c| !c.is_whitespace())?;
    Some(tui.foreground(u16::try_from(row).ok()?, u16::try_from(col).ok()?))
}

/// 流光把颜色压暗、提亮，色相不变：照离哪个近算。调色板色照原样画，只能一样。
fn nearer(got: Color, want: Color, other: Color) -> bool {
    match (got, want, other) {
        (Color::Rgb(r, g, b), Color::Rgb(wr, wg, wb), Color::Rgb(or, og, ob)) => {
            let d = |x: (u8, u8, u8)| {
                [(r, x.0), (g, x.1), (b, x.2)]
                    .iter()
                    .map(|(a, b)| (i32::from(*a) - i32::from(*b)).pow(2))
                    .sum::<i32>()
            };
            d((wr, wg, wb)) < d((or, og, ob))
        }
        _ => got == want,
    }
}

fn wait<T>(tui: &mut Tui, what: &str, mut got: impl FnMut(&Tui) -> Option<T>) -> T {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(value) = got(tui) {
            return value;
        }
        assert!(
            Instant::now() < end,
            "等不到{what}：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(100));
    }
}

#[test]
fn the_pulse_keeps_the_workspace_colour_in_every_theme() {
    for (colors, mode, workspace) in THEMES {
        let theme = format!("{colors} {mode}");
        let home = Home::with_settings(
            Script::new([Play::Holds]),
            &format!("[tui]\ntheme = \"{colors}\"\ntheme_mode = \"{mode}\"\n"),
        );
        home.persona(
            "green",
            "[persona]\nname = \"绿\"\n\n[appearance]\nseed = \"#2fab34\"\n",
            None,
        );
        let mut tui = home.tui_with("zh_CN.UTF-8", &[("MIYU_TUI_PERSONA", "green")]);
        let full = wait(&mut tui, "「开放权限」", |t| {
            footer_color(t, "开放权限")
        });
        assert!(
            nearer(full, workspace, RED),
            "{theme}：开放权限 {full:?} 要照工作区 {workspace:?}，不是红的"
        );
        tui.say("你好");
        tui.wait_for_line("…");
        let pulse = wait(&mut tui, "运行状态行", pulse_color);
        assert!(
            nearer(pulse, workspace, RED),
            "{theme}：流光 {pulse:?} 要照工作区 {workspace:?}\n{}",
            tui.lines().join("\n")
        );
    }
}
