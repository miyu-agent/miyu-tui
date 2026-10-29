//! 伪终端里的端到端测试：自选主色（蓝图 `tui.md`「主题」的「主色」，2026-10-14 项目主人定）。头像的候选由核心算、人格信息里
//! 给；外观页「主色」那一格开色块列表，选了写进人格（`persona.set` 的 `seed`），两个头看到的是同一个。

mod support;

use miyu_session::testkit::Script;
use support::Home;
use vt100::Color;

/// tokyonight 深色那一套铺的底色。
const NIGHT: Color = Color::Rgb(0x1a, 0x1b, 0x26);

/// 放一个人格，头像是四块颜色（红、绿、蓝、黄）：核心照它挑出四个候选。
fn four_hues(home: &Home) {
    home.persona("duo", "[persona]\nname = \"双色\"\n", None);
    let image = image::RgbaImage::from_fn(96, 128, |x, y| {
        let [r, g, b] = [[200, 50, 60], [60, 170, 80], [50, 90, 200], [230, 200, 60]]
            [usize::try_from((y / 64) * 2 + x / 48).unwrap_or(0)];
        image::Rgba([r, g, b, 255])
    });
    let file = home.root().join("personas/duo/avatar.png");
    image.save(&file).expect("写得进头像");
}

/// 等左上角那一格铺的底色满足 `ok`。
fn wait_ground(tui: &mut support::Tui, ok: &dyn Fn(Color) -> bool, what: &str) -> Color {
    let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !ok(tui.background(0, 0)) {
        assert!(
            std::time::Instant::now() < end,
            "{what}：{:?}\n{}",
            tui.background(0, 0),
            tui.lines().join("\n")
        );
        tui.pump(std::time::Duration::from_millis(100));
    }
    tui.background(0, 0)
}

#[test]
fn the_main_color_is_picked_from_the_avatars_candidates_and_saved_on_the_persona() {
    let settings = "[tui]\ntheme = \"persona\"\ntheme_mode = \"dark\"\n";
    let home = Home::with_settings(Script::new([]), settings);
    four_hues(&home);
    let mut tui = home.tui_with("zh_CN.UTF-8", &[("MIYU_TUI_PERSONA", "duo")]);
    tui.wait_for("工作区");
    // 没选过：照打分最高的那个铺。
    let first = wait_ground(
        &mut tui,
        &|c| c != NIGHT && matches!(c, Color::Rgb(..)),
        "照头像铺",
    );
    tui.say("/config");
    tui.open_menu("外观");
    tui.wait_for("主色");
    // 外观页第一项是「主题」（2026-10-21），下一格配色，再下面一格就是主色：开色块列表，四个候选，第一个写「默认」。
    tui.key(b"j");
    tui.key(b"j");
    tui.key(b"\r");
    tui.wait_for("默认");
    let swatches = tui
        .lines()
        .iter()
        .filter(|l| l.trim_start().starts_with("██ #"))
        .count();
    assert_eq!(swatches, 4, "{}", tui.lines().join("\n"));
    // 挑第二个：写进人格的主题色，颜色当场换。
    tui.key(b"j");
    tui.key(b"\r");
    let toml = home.root().join("personas/duo/persona.toml");
    let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !std::fs::read_to_string(&toml)
        .unwrap_or_default()
        .contains("seed = \"#")
    {
        assert!(std::time::Instant::now() < end, "写进人格的主题色");
        tui.pump(std::time::Duration::from_millis(100));
    }
    wait_ground(&mut tui, &|c| c != first && c != NIGHT, "换成选的那个");
    // `d` 恢复默认：清掉人格的主题色，回到打分最高的那个。
    tui.key(b"d");
    tui.wait_for("恢复默认：主色？");
    tui.key(b"h");
    tui.key(b"\r");
    let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::fs::read_to_string(&toml)
        .unwrap_or_default()
        .contains("seed = ")
    {
        assert!(std::time::Instant::now() < end, "清掉人格的主题色");
        tui.pump(std::time::Duration::from_millis(100));
    }
    wait_ground(&mut tui, &|c| c == first, "回到打分最高的那个");
}

#[test]
fn the_main_color_item_shows_only_while_following_an_avatar_or_wallpaper() {
    let home = Home::with_settings(Script::new([]), "[tui]\ntheme = \"tokyonight\"\n");
    four_hues(&home);
    let mut tui = home.tui_args_with("zh_CN.UTF-8", &["config"], &[("MIYU_TUI_PERSONA", "duo")]);
    tui.open_menu("外观");
    tui.wait_for("配色");
    assert!(!tui.shows("主色"), "{}", tui.lines().join("\n"));
}
