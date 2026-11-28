//! 窗口矮到一两行、零行（审计 2026-11-28 第 9 条）：原来面包屑、包页面作者那一行、弹窗标题照「上边加几行」写到缓冲区外，
//! `set_stringn` 不查纵坐标，一画就 panic。

use ratatui::crossterm::event::KeyCode;

use super::{page, press, screen};
use crate::config::Config;
use crate::settings::pages::Section;
use crate::settings::pages::packages::Origin;

const TINY: [(u16, u16); 7] = [
    (80, 1),
    (3, 1),
    (120, 1),
    (120, 0),
    (40, 0),
    (120, 2),
    (60, 3),
];

#[test]
fn the_menu_and_the_models_page_draw_in_one_or_zero_rows() {
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    for (w, h) in TINY {
        screen(&mut p, &config, w, h);
    }
    press(&mut p, &config, KeyCode::Enter);
    for (w, h) in TINY {
        screen(&mut p, &config, w, h);
    }
}

#[test]
fn a_package_page_with_its_origin_row_draws_in_two_or_three_rows() {
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    p.on_menu = false;
    p.more.section = Some(Section::Page("packages/say".into(), 0));
    p.more.origins.insert(
        "say".into(),
        Origin {
            author: Some("someone".into()),
            from: None,
            upstream: Some("https://example.invalid/say".into()),
        },
    );
    let (text, _) = screen(&mut p, &config, 120, 30);
    assert!(text.contains("someone"), "作者那一行画出来了：\n{text}");
    for (w, h) in TINY {
        screen(&mut p, &config, w, h);
    }
}

#[test]
fn an_open_popup_draws_in_one_or_zero_rows() {
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    press(&mut p, &config, KeyCode::Enter);
    press(&mut p, &config, KeyCode::Down);
    press(&mut p, &config, KeyCode::Enter);
    assert!(p.popup.is_some(), "开了供应商的窗");
    for (w, h) in TINY {
        screen(&mut p, &config, w, h);
    }
    // 放大回来照原样画。
    let (text, _) = screen(&mut p, &config, 120, 34);
    assert!(!text.trim().is_empty());
}
