use ratatui::style::Color;

use super::{seeded, seeds, terminal};
use crate::theme::Palette;
use crate::theme::builtin;
use crate::theme::terminal_colors::{Colors, Ground, Seen};

fn named(name: &str) -> Palette {
    let themes = builtin().unwrap();
    themes.into_iter().find(|(n, _)| n == name).unwrap().1
}

fn base() -> Palette {
    named("terminal")
}

const DARK: Ground = Ground {
    fg: (200, 200, 200),
    bg: (20, 20, 20),
    dark: true,
};

/// 问到的：`truecolor` 是不是真彩色。
fn seen(ground: Ground, sixteen: Option<Colors>, truecolor: bool) -> Seen {
    Seen {
        sixteen,
        ground: Some(ground),
        truecolor,
    }
}

/// 灰的 16 色，`n` 号换成 `color`。
fn gray_but(pairs: &[(usize, (u8, u8, u8))]) -> Colors {
    let mut sixteen = [(128, 128, 128); 16];
    for &(n, color) in pairs {
        sixteen[n] = color;
    }
    sixteen
}

#[test]
fn panels_lean_from_the_terminals_ground_toward_its_text() {
    // 2026-10-14 项目主人照推荐定：面板、差异这些底色照终端的底色推，不再是写死的 256 色序号。
    let sixteen = gray_but(&[(1, (220, 40, 40)), (2, (40, 220, 40))]);
    let out = terminal(&base(), seen(DARK, Some(sixteen), false));
    let Color::Rgb(r, g, b) = out.panel_bg.0 else {
        panic!("{:?}", out.panel_bg)
    };
    assert!(
        (25..=40).contains(&r) && r == g && g == b,
        "比底亮一点的灰：{r}"
    );
    let Color::Rgb(_, g, _) = out.diff_added_bg.0 else {
        panic!()
    };
    let Color::Rgb(r, _, _) = out.diff_removed_bg.0 else {
        panic!()
    };
    assert!(g > 40 && r > 40, "差异的底往绿、红掺了一点");
    assert_eq!(out.diagram_backdrop.0, Color::Rgb(20, 20, 20));
    // 不是真彩色：字照主题文件写的终端 16 色名字，不动。
    assert_eq!(out.accent, base().accent);
}

#[test]
fn without_sixteen_colors_the_diff_grounds_stay_as_the_theme_file_says() {
    let light = Ground {
        fg: (40, 40, 40),
        bg: (240, 240, 240),
        dark: false,
    };
    let out = terminal(&base(), seen(light, None, true));
    assert_eq!(out.diff_added_bg, base().diff_added_bg);
    let Color::Rgb(r, ..) = out.panel_bg.0 else {
        panic!()
    };
    assert!(r < 240 && r > 200, "浅色的底往深的字掺一点：{r}");
}

#[test]
fn the_most_vivid_of_the_sixteen_comes_first_only_on_a_truecolor_terminal() {
    // 2026-10-19 项目主人定「挑最鲜艳的、可换」：带颜色的 12 号照跟随壁纸同一套打分；黑白灰那几号不算。
    let sixteen = gray_but(&[(0, (255, 0, 255)), (4, (30, 90, 250)), (9, (150, 120, 115))]);
    let got = seeds(seen(DARK, Some(sixteen), true));
    assert!(!got.is_empty());
    assert_eq!(
        got[0], 0x1e_5afa,
        "蓝排第一，就是终端那一号的字节：{got:x?}"
    );
    assert!(!got.contains(&0xff_00ff), "0 号（黑）不算，哪怕写成了品红");
    // 全是黑白灰：打分给的 Google 蓝不是终端的，没有候选。
    assert!(seeds(seen(DARK, Some(gray_but(&[])), true)).is_empty());
    // 不是真彩色、没问到 16 色、没问到字色底色：没有候选，照主题文件回退。
    assert!(seeds(seen(DARK, Some(sixteen), false)).is_empty());
    assert!(seeds(seen(DARK, None, true)).is_empty());
    let unground = Seen {
        sixteen: Some(sixteen),
        ground: None,
        truecolor: true,
    };
    assert!(seeds(unground).is_empty());
}

#[test]
fn a_seeded_palette_keeps_its_colors_but_takes_the_terminals_ground() {
    // 2026-10-19 项目主人定「照旧用终端的背景」：字、强调色照 MD3，面板这些底从终端的底往生成的正文色推。
    let md3 = crate::theme::persona::overlay(&named("tokyonight-night"), 0x1e_5afa, true);
    let out = seeded(md3.clone(), seen(DARK, None, true));
    assert_eq!(out.accent, md3.accent);
    assert_eq!(out.text, md3.text);
    assert_eq!(out.diagram_backdrop.0, Color::Rgb(20, 20, 20));
    assert_ne!(out.panel_bg, md3.panel_bg);
    let Color::Rgb(r, g, b) = out.panel_bg.0 else {
        panic!("{:?}", out.panel_bg)
    };
    assert!(
        r.max(g).max(b) < 50 && r.max(g).max(b) > 20,
        "比终端的底亮一点：{r} {g} {b}"
    );
    let Color::Rgb(tr, tg, tb) = md3.text.0 else {
        panic!("{:?}", md3.text)
    };
    assert_eq!(
        (r, g, b),
        super::mix((20, 20, 20), (tr, tg, tb), super::PANEL),
        "往生成的正文色掺，不往终端的字色掺"
    );
    // 没问到底色的原样。
    let bare = Seen {
        sixteen: None,
        ground: None,
        truecolor: true,
    };
    assert_eq!(seeded(md3.clone(), bare), md3);
}
