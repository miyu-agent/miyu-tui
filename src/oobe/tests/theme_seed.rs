//! 主题那一步的主色、铺底色（2026-10-15 项目主人：配色方案和主色，铺底色也缺了）：主色只在跟随头像、跟随壁纸、终端配色、有候选时
//! 列，挑了当场换，跟随壁纸的写 `tui.theme_seed`，跟随头像的写完配置再写回这个人格；铺底色关了当场换、写进设置。

use std::time::Instant;

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::theme::{at_theme, defaults};
use super::{asks, find, fresh, key, texts};
use crate::oobe::theme::Slot;
use crate::oobe::{Oobe, Step, Texts};

/// 光标挪到 `slot` 那一格。
fn go(oobe: &mut Oobe, slot: Slot, texts: &Texts, t0: Instant) {
    for _ in 0..8 {
        if oobe.theme.slot == slot {
            return;
        }
        oobe.key(key(KeyCode::Down), t0, texts);
    }
    assert_eq!(oobe.theme.slot, slot);
}

#[test]
fn turning_the_background_off_shows_at_once_and_is_written() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    at_theme(&mut oobe, t0, &texts, defaults());
    assert!(
        oobe.theme_preview().is_some_and(|p| p.ground),
        "没写过的照默认开"
    );
    go(&mut oobe, Slot::Ground, &texts, t0);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(key(KeyCode::Down), t0, &texts);
    assert!(
        oobe.theme_preview().is_some_and(|p| !p.ground),
        "挪到关当场不铺"
    );
    oobe.key(key(KeyCode::Enter), t0, &texts);
    go(&mut oobe, Slot::Next, &texts, t0);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (_, params) = find(&asks(&mut oobe), "config.set");
    assert!(
        params["changes"]
            .as_array()
            .is_some_and(|c| c.contains(&json!({"key": "tui.theme_background", "value": false}))),
        "{params}"
    );
}

#[test]
fn the_seed_row_follows_the_colors_and_writes_where_the_colors_come_from() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    let mut items = defaults();
    items["tui.theme"] = json!({"value": "wallpaper", "origin": {"layer": "personal"}});
    at_theme(&mut oobe, t0, &texts, items);
    assert!(
        !oobe.theme.slots().contains(&Slot::Seed),
        "还没有候选的不列"
    );
    // App 交来壁纸的候选：列主色，画正用着的。
    oobe.theme.seeds = vec![0x11_22_33, 0x44_55_66];
    oobe.theme.seed = Some(0x11_22_33);
    assert!(oobe.theme.slots().contains(&Slot::Seed));
    go(&mut oobe, Slot::Seed, &texts, t0);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let choose = oobe.theme.picking.as_ref().expect("开了色块列表");
    assert!(choose.swatches, "色块列表");
    // 挪到第二个：当场换；Esc 换回没挑的。
    oobe.key(key(KeyCode::Down), t0, &texts);
    assert_eq!(oobe.theme_preview().and_then(|p| p.seed), Some(0x44_55_66));
    oobe.key(key(KeyCode::Esc), t0, &texts);
    assert_eq!(oobe.theme_preview().and_then(|p| p.seed), None);
    // 再挑第二个，「下一步」写个人设置的 tui.theme_seed。
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(key(KeyCode::Down), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert_eq!(oobe.theme.shown_seed(), Some(0x44_55_66));
    go(&mut oobe, Slot::Next, &texts, t0);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, params) = find(&asks(&mut oobe), "config.set");
    assert!(
        params["changes"]
            .as_array()
            .is_some_and(|c| c.contains(&json!({"key": "tui.theme_seed", "value": "#445566"}))),
        "{params}"
    );
    oobe.answer(tag, Ok(json!({})), t0, &texts);
    assert_eq!(oobe.step, Step::Profile, "跟随壁纸的写完配置就往下走");
}

#[test]
fn a_seed_picked_while_following_the_avatar_is_written_to_the_persona() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    oobe.persona.chosen_id = Some("persona-1".into());
    let mut items = defaults();
    items["tui.theme"] = json!({"value": "persona", "origin": {"layer": "personal"}});
    at_theme(&mut oobe, t0, &texts, items);
    oobe.theme.seeds = vec![0xaa_bb_cc, 0x12_34_56];
    oobe.theme.seed = Some(0xaa_bb_cc);
    go(&mut oobe, Slot::Seed, &texts, t0);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(key(KeyCode::Down), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    go(&mut oobe, Slot::Next, &texts, t0);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, params) = find(&asks(&mut oobe), "config.set");
    assert!(
        !params.to_string().contains("tui.theme_seed"),
        "跟随头像的不写个人设置的主色：{params}"
    );
    oobe.answer(tag, Ok(json!({})), t0, &texts);
    assert_eq!(oobe.step, Step::Theme, "还要写人格");
    let (tag, params) = find(&asks(&mut oobe), "persona.set");
    assert_eq!(params, json!({"persona": "persona-1", "seed": "#123456"}));
    oobe.answer(tag, Ok(json!({})), t0, &texts);
    assert_eq!(oobe.step, Step::Profile);
}

#[test]
fn terminal_colors_hide_the_background_row_and_keep_its_value() {
    // 2026-10-15 项目主人定：配色是终端配色时不铺，「铺底色」这一格不列，永远露出终端自己的底；换走了再出来，值照原来的。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    let mut items = defaults();
    items["tui.theme"] = json!({"value": "terminal", "origin": {"layer": "personal"}});
    items["tui.theme_background"] = json!({"value": false, "origin": {"layer": "personal"}});
    at_theme(&mut oobe, t0, &texts, items);
    assert!(
        !oobe.theme.slots().contains(&Slot::Ground),
        "{:?}",
        oobe.theme.slots()
    );
    // 换成 Tokyo Night：这一格出来，还是关。
    oobe.theme.colors = crate::theme::choice::COLORS
        .iter()
        .position(|c| *c == "tokyonight")
        .unwrap();
    assert!(oobe.theme.slots().contains(&Slot::Ground));
    assert!(
        oobe.theme_preview().is_some_and(|p| !p.ground),
        "值照原来的"
    );
}

#[test]
fn terminal_colors_with_candidates_list_the_seed_row_and_write_the_pick() {
    // 2026-10-19 项目主人定：终端配色照终端 16 色里的主色生成 MD3，主色能换，写个人设置的 tui.theme_seed。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    let mut items = defaults();
    items["tui.theme"] = json!({"value": "terminal", "origin": {"layer": "personal"}});
    at_theme(&mut oobe, t0, &texts, items);
    assert!(
        !oobe.theme.slots().contains(&Slot::Seed),
        "没候选（不是真彩色）不列"
    );
    oobe.theme.seeds = vec![0x1e_5afa, 0xe6_2828];
    oobe.theme.seed = Some(0x1e_5afa);
    assert!(oobe.theme.slots().contains(&Slot::Seed));
    go(&mut oobe, Slot::Seed, &texts, t0);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(key(KeyCode::Down), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    go(&mut oobe, Slot::Next, &texts, t0);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (_, params) = find(&asks(&mut oobe), "config.set");
    assert!(
        params["changes"]
            .as_array()
            .is_some_and(|c| c.contains(&json!({"key": "tui.theme_seed", "value": "#e62828"}))),
        "{params}"
    );
}
