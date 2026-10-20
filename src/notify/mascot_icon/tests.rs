//! 通知图标的像素图：晨光的颜色、一圈轮廓、放大不插值，和网页的吉祥物一格不差。

use super::{Icon, MORNING, SCALE, draw, file};
use crate::config::Config;

#[test]
fn the_mascot_is_pixel_art_in_the_morning_colors_with_an_outline() {
    // 2026-10-19 项目主人：照网页晨光主题那一只、符号化一些。
    let look = Config::builtin().unwrap().mascot;
    let picture = draw(&look, Icon::Standing);
    assert_eq!(picture.width(), 26 * SCALE, "24 格连轮廓，放大不插值");
    let has = |rgb: [u8; 3]| {
        picture
            .pixels()
            .any(|p| p.0 == [rgb[0], rgb[1], rgb[2], 255])
    };
    assert!(has(MORNING.outline), "一圈轮廓");
    assert!(has(MORNING.eye), "眼睛照原色");
    assert!(has(MORNING.head), "头最亮的那一档就是晨光的奶白");
    assert!(has(MORNING.fin) || has(MORNING.fin_shade), "鳍是粉的");
    assert_eq!(picture.get_pixel(0, 0).0[3], 0, "角上透明");
    // 放大不插值：每个格子里的像素颜色都一样。
    let colors: std::collections::BTreeSet<[u8; 4]> = picture.pixels().map(|p| p.0).collect();
    assert!(colors.len() <= 20, "只有几种颜色，像素画：{}", colors.len());
    for y in (0..picture.height()).step_by(SCALE as usize) {
        for x in (0..picture.width()).step_by(SCALE as usize) {
            let first = picture.get_pixel(x, y);
            assert_eq!(picture.get_pixel(x + SCALE - 1, y + SCALE - 1), first);
        }
    }
}

#[test]
fn the_pixel_face_is_used_when_the_mascot_has_one() {
    // 2026-10-19 项目主人：照字符画那一份的脸画，肚子上的圈大成一个「H」，丑；像素图照 `pixel_face`。
    let look = Config::builtin().unwrap().mascot;
    assert!(look.pixel_face.is_some(), "内置的吉祥物写了像素图用的脸");
    let without = crate::mascot::Look {
        pixel_face: None,
        ..look.clone()
    };
    assert_ne!(
        draw(&look, Icon::Standing),
        draw(&without, Icon::Standing),
        "照另调的那一份画"
    );
}

#[test]
fn a_notification_in_progress_holds_a_laptop_and_a_finished_one_does_not() {
    // 2026-10-19 项目主人：过程中的通知用玩电脑的，完成的用站着的。
    let look = Config::builtin().unwrap().mascot;
    let lamp = [MORNING.lamp[0], MORNING.lamp[1], MORNING.lamp[2], 255];
    let working = draw(&look, Icon::Working);
    let standing = draw(&look, Icon::Standing);
    assert!(working.pixels().any(|p| p.0 == lamp), "电脑盖子上的灯");
    assert!(!standing.pixels().any(|p| p.0 == lamp), "站着的没有电脑");
    assert_eq!(working.dimensions(), standing.dimensions());
    let dir = std::env::temp_dir().join(format!("miyu-mascot-laptop-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    assert_ne!(
        file(&look, Icon::Working, &dir),
        file(&look, Icon::Standing, &dir),
        "两张分开存"
    );
    drop(std::fs::remove_dir_all(&dir));
}

#[test]
fn one_look_is_drawn_once() {
    let look = Config::builtin().unwrap().mascot;
    let dir = std::env::temp_dir().join(format!("miyu-mascot-icon-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    let first = file(&look, Icon::Standing, &dir).expect("画得出来");
    let stamp = std::fs::metadata(&first).unwrap().modified().unwrap();
    assert_eq!(file(&look, Icon::Standing, &dir), Some(first.clone()));
    assert_eq!(
        std::fs::metadata(&first).unwrap().modified().unwrap(),
        stamp,
        "不重画"
    );
    drop(std::fs::remove_dir_all(&dir));
}

/// 网页 `packages/mascot`（miyu-web `mascot-eyes` 5e8bd89：眼睛是竖着的长方形）照晨光画出来的那一张，一格一个字（node 照网页的 `render`、`paint`、
/// `rgbOf`、`props` 画了再照下面 [`code`] 的表认）：`.` 透明，`o` 轮廓，`E` 眼睛，`L` 锯齿线和肚子上的圈，`G` 灯，`0`–`3` 头、
/// `4`–`7` 耳朵、`8`–`B` 鳍从最暗到最亮。
const WEB_STANDING: [&str; 26] = [
    "..........................",
    "..........................",
    ".........o......o.........",
    "........o6o....o6o........",
    "........o76o..o76o........",
    "........o77oooo76o........",
    ".........o761166o.........",
    "........o22222211o........",
    ".......o2223322221o.......",
    "......o223EE33EE221o......",
    ".....o2233EE33EE2211o.....",
    "....o12333EE33EE22210o....",
    "....o2233333333322210o....",
    "....o2L333333332222L0o....",
    "....o2LL3333LL3222LL0o....",
    "....oL2L33LL33LL22L1Lo....",
    "....o122LL333222LL110o....",
    "....o1222222LL2221100o....",
    ".....o12222L22L21110o.....",
    "......o1122L22L1110o......",
    ".......o1111LL1100o.......",
    ".......oA111110019o.......",
    "......oBBBA0000BBBAo......",
    ".....oBBAAooooooABBBo.....",
    "....oBA99o......o9AAAo....",
    ".....oooo........oooo.....",
];

/// 同上，抱着电脑的：电脑比网页抬高两格（[`super::laptop::RAISE`]，2026-10-20 项目主人：「把电脑往上移动一些，然后嘴巴遮住大部分」）。
const WEB_WORKING: [&str; 26] = [
    "..........................",
    "..........................",
    ".........o......o.........",
    "........o6o....o6o........",
    "........o76o..o76o........",
    "........o77oooo76o........",
    ".........o761166o.........",
    "........o22222211o........",
    ".......o2223322221o.......",
    "......o223EE33EE221o......",
    ".....o2233EE33EE2211o.....",
    "....o12333EE33EE22210o....",
    "....o2233333333322210o....",
    "....o2L333333332222L0o....",
    "....o2oooooooooooooo0o....",
    "....oLo444444444444oLo....",
    "....o1o444444444444o0o....",
    "....o1o44444GG44444o0o....",
    ".....oo4444GGGG4444oo.....",
    "......o44444GG44444o......",
    "......o444444444444o......",
    ".....oooooooooooooooo.....",
    ".....o77777777777777o.....",
    ".....oooooooooooooooo.....",
    "....oBA99o......o9AAAo....",
    ".....oooo........oooo.....",
];

/// 一个颜色认成 [`WEB_STANDING`] 里的哪个字。
fn code(rgb: [u8; 4]) -> char {
    if rgb[3] == 0 {
        return '.';
    }
    let p = &MORNING;
    let rgb = [rgb[0], rgb[1], rgb[2]];
    let line = super::mix(p.head_shade, p.outline, super::LINE_SHADE);
    let fixed = [('o', p.outline), ('E', p.eye), ('L', line), ('G', p.lamp)];
    if let Some((c, _)) = fixed.iter().find(|(_, v)| *v == rgb) {
        return *c;
    }
    let parts = [
        (p.head_shade, p.head),
        (p.ear_shade, p.ear),
        (p.fin_shade, p.fin),
    ];
    for (i, (shade, light)) in parts.iter().enumerate() {
        for level in 0..4 {
            if super::mix(*shade, *light, level as f64 / 3.0) == rgb {
                return char::from_digit((i * 4 + level) as u32, 16)
                    .unwrap()
                    .to_ascii_uppercase();
            }
        }
    }
    '?'
}

/// 画出来的那一张一格一个字。
fn codes(icon: Icon) -> Vec<String> {
    let picture = draw(&Config::builtin().unwrap().mascot, icon);
    let side = picture.width() / SCALE;
    (0..side)
        .map(|y| {
            (0..side)
                .map(|x| code(picture.get_pixel(x * SCALE, y * SCALE).0))
                .collect()
        })
        .collect()
}

#[test]
fn it_is_the_web_mascot_cell_for_cell() {
    // 2026-10-20 项目主人：「感觉图标还是怪怪的，你真的是照着webui的那个做的吗？重新做一下。」原来借终端字符画那一套算，明暗、
    // 耳根的轮廓、眼睛（网页 10-19 改回两格宽）都和网页对不上。照网页的算法画（`pixel.rs`），一格不差；抱着电脑的照网页，电脑
    // 抬高两格，盖住嘴的大半（同一天项目主人又定）；眼睛照网页改成竖着的长方形（同一天项目主人：「双竖线，长方形」）。
    assert_eq!(codes(Icon::Standing), WEB_STANDING, "站着的");
    assert_eq!(codes(Icon::Working), WEB_WORKING, "抱着电脑的");
}

/// 画出来看（`ICON_OUT=<目录> cargo test dump_icons -- --ignored`）：调吉祥物的格子时用。
#[test]
#[ignore]
fn dump_icons() {
    let look = Config::builtin().unwrap().mascot;
    let out = std::path::PathBuf::from(std::env::var("ICON_OUT").unwrap());
    draw(&look, Icon::Standing)
        .save(out.join("standing.png"))
        .unwrap();
    draw(&look, Icon::Working)
        .save(out.join("working.png"))
        .unwrap();
}
