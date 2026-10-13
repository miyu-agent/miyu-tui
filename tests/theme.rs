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
