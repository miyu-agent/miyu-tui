//! 挑主题、主色的测试。

use ratatui::style::Color;
use serde_json::json;

use super::{Choice, Mode, pick};
use crate::theme::builtin;

#[test]
fn a_picked_main_color_holds_only_while_it_is_still_a_candidate() {
    // 2026-10-14 项目主人定：跟随壁纸自选的主色在候选里照它；换了壁纸候选整个换了，回到打分最高的那个（和 Android 一样）。
    let seeds = [0xd1_2b33, 0x36_b446];
    assert_eq!(super::pick_seed(&seeds, Some(0x36_b446)), Some(0x36_b446));
    assert_eq!(
        super::pick_seed(&seeds, Some(0x12_3456)),
        Some(0xd1_2b33),
        "不在候选里"
    );
    assert_eq!(super::pick_seed(&seeds, None), Some(0xd1_2b33), "没选");
    assert_eq!(super::pick_seed(&[], Some(0x36_b446)), None, "没有候选");
}

#[test]
fn following_the_wallpaper_colors_like_following_an_avatar_with_the_same_seed() {
    // 2026-10-14 项目主人定：跟随壁纸和跟随头像同一套算法，同一个主色出一样的颜色；没找到壁纸（没有主色）的照 tokyonight。
    let themes = builtin().unwrap();
    let choice = |colors: &str| Choice {
        colors: colors.into(),
        mode: Mode::Dark,
        ground: true,
    };
    let wallpaper = pick(&choice("wallpaper"), None, &themes, Some(0x2f_ab34)).unwrap();
    let persona = pick(&choice("persona"), None, &themes, Some(0x2f_ab34)).unwrap();
    assert_eq!(wallpaper.name, "wallpaper-night");
    assert_eq!(wallpaper.palette.background, persona.palette.background);
    assert_eq!(wallpaper.palette.thought, persona.palette.thought);
    let missing = pick(&choice("wallpaper"), None, &themes, None).unwrap();
    assert_eq!(missing.name, "tokyonight-night");
}

#[test]
fn terminal_colors_generate_md3_from_the_terminals_main_color_on_truecolor() {
    // 2026-10-19 项目主人：跟随终端时用终端 16 色里的主色生成 MD3，不直接用 16 色；不是真彩色才回退。底照旧用终端的。
    use crate::theme::terminal_colors::{Ground, Seen};
    let themes = builtin().unwrap();
    let terminal = Choice {
        colors: "terminal".into(),
        mode: Mode::Dark,
        ground: true,
    };
    let mut sixteen = [(128, 128, 128); 16];
    sixteen[4] = (30, 90, 250);
    sixteen[1] = (230, 40, 40);
    let seen = |truecolor| Seen {
        sixteen: Some(sixteen),
        ground: Some(Ground {
            fg: (200, 200, 200),
            bg: (20, 20, 20),
            dark: true,
        }),
        truecolor,
    };
    let first = crate::theme::derived::seeds(seen(true));
    assert!(first.len() >= 2, "{first:x?}");
    let picked = super::pick_in(&terminal, None, &themes, None, seen(true)).unwrap();
    assert_eq!(picked.name, "terminal-night");
    assert!(picked.ground.is_none(), "不铺底色");
    let md3 = pick(&choice_of("wallpaper"), None, &themes, Some(first[0])).unwrap();
    assert_eq!(
        picked.palette.accent, md3.palette.accent,
        "没挑过：打分最高的"
    );
    assert!(
        matches!(picked.palette.text.0, Color::Rgb(..)),
        "字是生成的真颜色：{:?}",
        picked.palette.text
    );
    assert_eq!(picked.palette.diagram_backdrop.0, Color::Rgb(20, 20, 20));
    // 挑过的在候选里照它；不在的照打分最高的。
    let other = super::pick_in(&terminal, None, &themes, Some(first[1]), seen(true)).unwrap();
    assert_ne!(other.palette.accent, picked.palette.accent);
    let stale = super::pick_in(&terminal, None, &themes, Some(0x12_3456), seen(true)).unwrap();
    assert_eq!(stale.palette.accent, picked.palette.accent);
    // 不是真彩色：照主题文件（字写终端 16 色的名字）。
    let plain = super::pick_in(&terminal, None, &themes, Some(first[0]), seen(false)).unwrap();
    assert_eq!(plain.name, "terminal");
    assert_eq!(plain.palette.accent.0, Color::Blue);
}

fn choice_of(colors: &str) -> Choice {
    Choice {
        colors: colors.into(),
        mode: Mode::Dark,
        ground: true,
    }
}

#[test]
fn the_settings_are_read_and_unknown_values_are_ignored() {
    let mut c = Choice::default();
    c.apply(&json!({"items": {
        "tui.theme": {"value": "terminal"},
        "tui.theme_mode": {"value": "light"},
        "tui.theme_background": {"value": false}
    }}));
    assert_eq!(
        (c.colors.as_str(), c.mode, c.ground),
        ("terminal", Mode::Light, false)
    );
    c.apply(
        &json!({"items": {"tui.theme": {"value": "pink"}, "tui.theme_mode": {"value": "dim"}}}),
    );
    assert_eq!(
        (c.colors.as_str(), c.mode),
        ("terminal", Mode::Light),
        "认不得的不动"
    );
}

#[test]
fn auto_follows_the_terminal_and_the_switch_paints_the_ground_in_every_mode() {
    // 2026-10-13 项目主人定：自动照终端的底色挑深浅，手动选的不看终端；铺底色能关。2026-10-15 项目主人定：三种明暗下
    // 开关意思一样，自动的开着也铺（原来自动不铺，引导里写着「开」却没铺）。
    let themes = builtin().unwrap();
    let auto = Choice::default();
    assert_eq!(
        pick(&auto, Some(false), &themes, None).unwrap().name,
        "tokyonight-day"
    );
    assert_eq!(
        pick(&auto, None, &themes, None).unwrap().name,
        "tokyonight-night",
        "问不到的当深色"
    );
    assert_eq!(
        pick(&auto, Some(false), &themes, None).unwrap().ground,
        Some((Color::Rgb(0xe1, 0xe2, 0xe7), Color::Rgb(0x37, 0x60, 0xbf))),
        "自动的开着也铺照终端挑出的那一套"
    );
    let auto_off = Choice {
        ground: false,
        ..Choice::default()
    };
    assert!(
        pick(&auto_off, Some(false), &themes, None)
            .unwrap()
            .ground
            .is_none(),
        "自动的关了不铺"
    );
    let light = Choice {
        mode: Mode::Light,
        ..Choice::default()
    };
    let picked = pick(&light, Some(true), &themes, None).unwrap();
    assert_eq!(picked.name, "tokyonight-day", "手动选的不看终端");
    assert_eq!(
        picked.ground,
        Some((Color::Rgb(0xe1, 0xe2, 0xe7), Color::Rgb(0x37, 0x60, 0xbf)))
    );
    let off = Choice {
        ground: false,
        ..light.clone()
    };
    assert!(
        pick(&off, None, &themes, None).unwrap().ground.is_none(),
        "关了不铺"
    );
    let terminal = Choice {
        colors: "terminal".into(),
        ..light
    };
    let picked = pick(&terminal, None, &themes, None).unwrap();
    assert_eq!(picked.name, "terminal-light");
    assert!(picked.ground.is_none(), "terminal 跟着终端的配色走，不铺");
}

#[test]
fn following_the_avatar_overlays_tokyonight_once_there_is_a_seed() {
    let themes = builtin().unwrap();
    let persona = Choice {
        colors: "persona".into(),
        mode: Mode::Dark,
        ..Choice::default()
    };
    let none = pick(&persona, None, &themes, None).unwrap();
    assert_eq!(none.name, "tokyonight-night", "还没有主色：照 tokyonight");
    let green = pick(&persona, None, &themes, Some(0x2fab34)).unwrap();
    assert_eq!(green.name, "persona-night");
    assert_ne!(green.palette.accent, none.palette.accent);
    assert_eq!(
        green.ground.map(|g| g.0),
        Some(green.palette.background.0),
        "铺的是生成的那一套的底色"
    );
    let tokyonight = Choice {
        colors: "tokyonight".into(),
        ..persona
    };
    assert_eq!(
        pick(&tokyonight, None, &themes, Some(0x2fab34))
            .unwrap()
            .name,
        "tokyonight-night",
        "不是跟随头像的不看主色"
    );
}

#[test]
fn seeds_follow_the_colors_without_the_screen() {
    // 全屏、shellhook 照同一个函数挑主色（2026-10-20 项目主人定 shellhook 和终端界面共用主题）。
    let wallpaper = [0xd1_2b33, 0x36_b446];
    assert_eq!(
        super::seeds("wallpaper", &wallpaper, None, Some(0x36_b446)),
        (wallpaper.to_vec(), Some(0x36_b446)),
        "跟随壁纸照自选的"
    );
    assert_eq!(
        super::seeds("wallpaper", &wallpaper, None, Some(0x12_3456)).1,
        Some(0xd1_2b33),
        "自选的不在候选里照打分最高的"
    );
    let avatar = ["#2fab34".to_string(), "#112233".to_string()];
    let persona = |seed| super::PersonaSeeds {
        seed,
        seeds: &avatar,
    };
    assert_eq!(
        super::seeds("persona", &wallpaper, Some(persona(None)), Some(0xd1_2b33)),
        (vec![0x2f_ab34, 0x11_2233], Some(0x2f_ab34)),
        "跟随头像不看壁纸、不看自选的"
    );
    assert_eq!(
        super::seeds("persona", &[], Some(persona(Some("#abcdef"))), None).1,
        Some(0xab_cdef),
        "人格写了主题色的照它"
    );
    assert_eq!(
        super::seeds("persona", &wallpaper, None, None),
        (Vec::new(), None)
    );
    assert_eq!(
        super::seeds("tokyonight", &wallpaper, None, Some(0xd1_2b33)),
        (Vec::new(), None)
    );
}
