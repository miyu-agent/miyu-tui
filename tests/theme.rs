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

#[test]
fn the_running_line_sweeps_a_real_gradient_when_the_terminal_tells_its_colors() {
    // 2026-10-14 项目主人：终端配色时流光只加粗变暗「怪怪的，是字变颜色，不是真的流光了」。启动时问到终端 16 色的真颜色
    // （OSC 4），流光照真的蓝算渐变：挨着的字字节不一样，都是那个蓝压暗、提亮出来的。
    let home = Home::with_settings(Script::new([Play::Holds]), "[tui]\ntheme = \"terminal\"\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    let mut colors = [(0x80, 0x80, 0x80); 16];
    colors[4] = (0x40, 0x80, 0xff);
    tui.answer_colors(colors);
    tui.wait_for("工作区");
    tui.say("在吗");
    tui.wait_for_line("…");
    let lines = tui.lines();
    let row = lines
        .iter()
        .rposition(|l| l.split_whitespace().last().is_some_and(seconds))
        .expect("运行状态行");
    let row = u16::try_from(row).unwrap();
    // 亮光一直在走：隔一会儿看一次整行，看到挨着的两个字颜色不一样就是渐变。中文占两格，后半格空着、没有颜色，跳过。
    let mut seen = Vec::new();
    for _ in 0..40 {
        let fg: Vec<Color> = (0..tui.lines()[usize::from(row)].chars().count())
            .filter_map(|col| u16::try_from(col).ok())
            .filter(|&col| !tui.cell(row, col).trim().is_empty())
            .map(|col| tui.foreground(row, col))
            .collect();
        for color in &fg {
            let Color::Rgb(r, g, b) = *color else {
                panic!("照真字节算：{color:?}\n{}", lines.join("\n"));
            };
            assert!(b >= g && g >= r, "还是终端的那个蓝：{r} {g} {b}");
        }
        if fg.windows(2).any(|w| w[0] != w[1]) {
            return;
        }
        seen.push(fg);
        tui.pump(std::time::Duration::from_millis(100));
    }
    panic!("一直是一个颜色，不是渐变：{seen:?}");
}

#[test]
fn terminal_colors_lean_the_selected_rows_ground_from_the_terminals_own() {
    // 2026-10-14 项目主人照推荐定：终端配色的面板、选中行这些底色照终端的底色往字色掺一点，不再是写死的 256 色序号。
    // 配置页主菜单选中的那一行铺 `row_bg`：底 (20,20,20) 往字 (200,200,200) 掺 18%。
    let home = Home::with_settings(Script::new([]), "[tui]\ntheme = \"terminal\"\n");
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    tui.answer_ground((200, 200, 200), (20, 20, 20));
    tui.answer_colors([(128, 128, 128); 16]);
    tui.wait_for("显示");
    // 读到 `tui.theme` 以前照出厂的 tokyonight 画（选中行是它的 #3d59a1），等配置读到、换成终端配色（10-14 CI 读早过一次）。
    let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let lines = tui.lines();
        let (row, col) = position(&lines, "通用").expect("主菜单的通用");
        let ground = tui.background(row, col);
        if ground == Color::Rgb(52, 52, 52) {
            break;
        }
        assert!(
            std::time::Instant::now() < end,
            "选中行的底照终端的底推：{ground:?}\n{}",
            lines.join("\n")
        );
        tui.pump(std::time::Duration::from_millis(100));
    }
}

#[test]
fn following_the_wallpaper_takes_matugens_source_like_an_avatar_color() {
    // 2026-10-14 项目主人定：跟随壁纸和网页同一个来处，matugen 生成的那一份优先，取它的 `source` 当主色，和跟随头像同一套
    // 算：同一个主色铺一样的底。
    let persona = ground("[tui]\ntheme = \"persona\"\ntheme_mode = \"dark\"\n");
    let settings = "[tui]\ntheme = \"wallpaper\"\ntheme_mode = \"dark\"\n";
    let home = Home::with_settings(Script::new([Play::Says("好。")]), settings);
    matugen(&home);
    let (root, local) = cache_dirs(&home);
    let env = [("HOME", root.as_str()), ("LOCALAPPDATA", local.as_str())];
    let mut tui = home.tui_with("zh_CN.UTF-8", &env);
    tui.wait_for("工作区");
    let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while tui.background(0, 0) != persona {
        assert!(
            std::time::Instant::now() < end,
            "跟随壁纸铺的底：{:?}，同一个主色跟随头像的：{persona:?}",
            tui.background(0, 0)
        );
        tui.pump(std::time::Duration::from_millis(100));
    }
}

#[test]
fn the_colors_list_offers_the_wallpaper_only_when_there_is_one() {
    // 2026-10-14 项目主人定：找不到壁纸的，配色里不列「跟随壁纸」。
    for found in [false, true] {
        let home = Home::with_settings(Script::new([]), "");
        if found {
            matugen(&home);
        }
        let (root, local) = cache_dirs(&home);
        let env = [("HOME", root.as_str()), ("LOCALAPPDATA", local.as_str())];
        let mut tui = home.tui_args_with("zh_CN.UTF-8", &["config"], &env);
        tui.open_menu("外观");
        tui.wait_for("配色");
        // 等起来时的那一次找完再开列表。
        tui.pump(std::time::Duration::from_millis(500));
        tui.key(b"\r");
        tui.wait_for("终端配色");
        assert_eq!(tui.shows("跟随壁纸"), found, "{}", tui.lines().join("\n"));
    }
}

#[test]
fn a_changed_wallpaper_is_followed_without_polling() {
    // 2026-10-14 项目主人：换了壁纸要自动跟着变，绝对不轮询：盯着壁纸来处的文件（`miyu-wallpaper` 的 `Watch`，系统的文件
    // 通知），一变就再找。这里换的是 matugen 那一份：不按键、不切焦点，底色自己换。
    let settings = "[tui]\ntheme = \"wallpaper\"\ntheme_mode = \"dark\"\n";
    let home = Home::with_settings(Script::new([]), settings);
    matugen(&home);
    let (root, local) = cache_dirs(&home);
    let env = [("HOME", root.as_str()), ("LOCALAPPDATA", local.as_str())];
    let mut tui = home.tui_with("zh_CN.UTF-8", &env);
    tui.wait_for("工作区");
    let wait_until = |tui: &mut support::Tui, ok: &dyn Fn(Color) -> bool, what: &str| {
        let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !ok(tui.background(0, 0)) {
            assert!(
                std::time::Instant::now() < end,
                "{what}：{:?}",
                tui.background(0, 0)
            );
            tui.pump(std::time::Duration::from_millis(100));
        }
    };
    wait_until(
        &mut tui,
        &|c| c != NIGHT && matches!(c, Color::Rgb(..)),
        "先照绿的铺",
    );
    let green = tui.background(0, 0);
    matugen_source(&home, "#b03060");
    wait_until(
        &mut tui,
        &|c| c != green && c != NIGHT,
        "matugen 那一份换了，底色跟着换",
    );
}

#[test]
fn without_file_notifications_only_regaining_focus_looks_again() {
    // 2026-10-14 项目主人：盯不了文件的（系统通知起不来）不退回定时去看，只在窗口拿回焦点时再找。不盯文件：换了 matugen
    // 那一份，等一阵底色不动（没有偷偷定时去看）；终端报拿回焦点（`ESC [ I`）才换。
    let settings = "[tui]\ntheme = \"wallpaper\"\ntheme_mode = \"dark\"\n";
    let home = Home::with_settings(Script::new([]), settings);
    matugen(&home);
    let (root, local) = cache_dirs(&home);
    let env = [
        ("HOME", root.as_str()),
        ("LOCALAPPDATA", local.as_str()),
        ("MIYU_TUI_WALLPAPER_WATCH", "0"),
    ];
    let mut tui = home.tui_with("zh_CN.UTF-8", &env);
    tui.wait_for("工作区");
    let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while tui.background(0, 0) == NIGHT || !matches!(tui.background(0, 0), Color::Rgb(..)) {
        assert!(std::time::Instant::now() < end, "先照绿的铺");
        tui.pump(std::time::Duration::from_millis(100));
    }
    let green = tui.background(0, 0);
    matugen_source(&home, "#b03060");
    tui.pump(std::time::Duration::from_secs(3));
    assert_eq!(tui.background(0, 0), green, "不盯文件的不自己去看");
    tui.send(b"\x1b[I");
    let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while tui.background(0, 0) == green {
        assert!(std::time::Instant::now() < end, "拿回焦点再找一次");
        tui.pump(std::time::Duration::from_millis(100));
    }
}

/// matugen 那一份在哪个缓存目录找（`miyu-wallpaper`）：Linux 照 `XDG_CACHE_HOME`（测具指在测试家目录的 `cache`），macOS
/// 照 `HOME/Library/Caches`，Windows 照 `LOCALAPPDATA`：后两样起界面时也指到测试家目录（交回 `HOME`、`LOCALAPPDATA`）。
fn cache_dirs(home: &Home) -> (String, String) {
    let root = home.root().display().to_string();
    let local = home.root().join("cache").display().to_string();
    (root, local)
}

/// 在测试家目录的缓存里放一份 matugen 生成的颜色（照 [`cache_dirs`] 指的地方），主色绿。
fn matugen(home: &Home) {
    matugen_source(home, "#2fab34");
}

/// 同 [`matugen`]，主色是 `source`。
fn matugen_source(home: &Home, source: &str) {
    let dir = if cfg!(target_os = "macos") {
        home.root().join("Library/Caches/miyu")
    } else {
        home.root().join("cache/miyu")
    };
    std::fs::create_dir_all(&dir).expect("建得了缓存目录");
    let colors = format!(r#"{{"source": "{source}", "light": {{}}, "dark": {{}}}}"#);
    std::fs::write(dir.join("matugen.json"), colors).expect("写得进去");
}

/// 一个词是读秒（`12s`）。
fn seconds(w: &str) -> bool {
    w.len() > 1 && w.ends_with('s') && w[..w.len() - 1].chars().all(|c| c.is_ascii_digit())
}

/// `text` 第一次出现在第几行、第几格（格照显示宽度数）。
fn position(lines: &[String], text: &str) -> Option<(u16, u16)> {
    lines.iter().enumerate().find_map(|(y, l)| {
        let at = l.find(text)?;
        let col = unicode_width::UnicodeWidthStr::width(&l[..at]);
        Some((u16::try_from(y).ok()?, u16::try_from(col).ok()?))
    })
}

#[test]
fn the_background_item_is_hidden_while_the_colors_follow_the_terminal() {
    // 2026-10-15 项目主人定：配色是终端配色时不铺，外观页不列「铺底色」，永远露出终端自己的底；别的配色照列。
    for (colors, listed) in [("terminal", false), ("tokyonight", true)] {
        let settings = format!("[tui]\ntheme = \"{colors}\"\n");
        let home = Home::with_settings(Script::new([]), &settings);
        let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
        tui.open_menu("外观");
        tui.wait_for("配色");
        tui.pump(std::time::Duration::from_millis(500));
        assert_eq!(tui.shows("铺底色"), listed, "{}", tui.lines().join("\n"));
    }
}
