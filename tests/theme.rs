//! 伪终端里的端到端测试：跟随头像的配色（蓝图 `tui.md`「主题」第 5 条，2026-10-13 项目主人定）。真界面连一份照剧本回话的
//! 核心，测试家目录里放一个写了主题色的人格。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;
use vt100::Color;

/// tokyonight 深色那一套铺的底色。
const NIGHT: Color = Color::Rgb(0x1a, 0x1b, 0x26);

/// 配色、深浅照 `settings` 设的，首页停在写了主题色的人格上，等连上、读完配置项，看左上角那一格铺的底色。
fn ground(settings: &str) -> Color {
    let home = Home::with_settings(Script::new([Play::Says("好。")]), settings);
    home.persona(
        "green",
        "[persona]\nname = \"绿\"\n\n[appearance]\nseed = \"#2fab34\"\n",
        None,
    );
    let mut tui = home.tui_with("zh_CN.UTF-8", &[("MIYU_TUI_PERSONA", "green")]);
    tui.wait_for("工作区");
    tui.pump(std::time::Duration::from_millis(800));
    tui.background(0, 0)
}

#[test]
fn following_the_avatar_paints_the_colors_of_the_viewed_persona() {
    let persona = ground("[tui]\ntheme = \"persona\"\ntheme_mode = \"dark\"\n");
    assert!(
        matches!(persona, Color::Rgb(..)) && persona != NIGHT,
        "跟随头像：照人格的主题色生成一套，铺它的底色：{persona:?}"
    );
    let tokyonight = ground("[tui]\ntheme = \"tokyonight\"\ntheme_mode = \"dark\"\n");
    assert_eq!(tokyonight, NIGHT, "选 tokyonight 的不看人格");
}

#[test]
fn the_running_line_sweeps_in_the_color_of_the_mode_word() {
    // 2026-10-14 项目主人报：终端配色时运行状态行的字是写死的蓝，和底栏「工作区」那个模式字对不上；照模式字的颜色走。
    let home = Home::with_settings(Script::new([Play::Holds]), "[tui]\ntheme = \"terminal\"\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("在吗");
    tui.wait_for_line("…");
    let lines = tui.lines();
    let mode = position(&lines, "工作区").expect("底栏的模式字");
    // 运行状态行：最后是读秒（`0s`），在输入框上面。
    let row = lines
        .iter()
        .rposition(|l| {
            l.split_whitespace().last().is_some_and(|w| {
                w.len() > 1
                    && w.ends_with('s')
                    && w[..w.len() - 1].chars().all(|c| c.is_ascii_digit())
            })
        })
        .expect("运行状态行");
    let col = lines[row].chars().take_while(|c| c.is_whitespace()).count();
    let word = tui.foreground(u16::try_from(row).unwrap(), u16::try_from(col).unwrap());
    let footer = tui.foreground(mode.0, mode.1);
    assert_eq!(
        word,
        footer,
        "流光和模式字同一个颜色：\n{}",
        lines.join("\n")
    );
}

/// `text` 第一次出现在第几行、第几格（格照显示宽度数）。
fn position(lines: &[String], text: &str) -> Option<(u16, u16)> {
    lines.iter().enumerate().find_map(|(y, l)| {
        let at = l.find(text)?;
        let col = unicode_width::UnicodeWidthStr::width(&l[..at]);
        Some((u16::try_from(y).ok()?, u16::try_from(col).ok()?))
    })
}
