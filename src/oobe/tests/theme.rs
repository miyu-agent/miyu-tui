//! 主题那一步（「第一次打开的引导」第 23a 条，2026-10-13 项目主人定；版面、默认停在哪 2026-10-14 定）。

use std::time::Instant;

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::{asks, find, fresh, key, texts};
use crate::oobe::field::Field;
use crate::oobe::theme::Slot;
use crate::oobe::{Move, Oobe, Step, Texts};
use crate::theme::choice::Mode;

/// 选着的配色、明暗。
pub(super) fn shown(oobe: &Oobe) -> Option<(String, Mode)> {
    oobe.theme_preview().map(|p| (p.colors, p.mode))
}

/// 建好人格以后从人格那一步走进来，读到的两项是 `items`。
pub(super) fn at_theme(oobe: &mut Oobe, t0: Instant, texts: &Texts, items: serde_json::Value) {
    if oobe.persona.chosen_id.is_none() {
        oobe.persona.chosen_id = Some("p1".into());
    }
    oobe.step = Step::Persona;
    oobe.moved(Move::Next, t0);
    assert_eq!(oobe.step, Step::Theme);
    let (tag, params) = find(&asks(oobe), "config.get");
    assert_eq!(
        params,
        json!({"keys": [
            "tui.theme", "tui.theme_mode", "tui.mascot_home", "tui.mascot_sidebar", "tui.theme_background"
        ]})
    );
    assert_eq!(shown(oobe), None, "读到以前照原来的样子");
    oobe.answer(tag, Ok(json!({"items": items})), t0, texts);
}

/// 出厂的两项：都是默认值。
pub(super) fn defaults() -> serde_json::Value {
    json!({
        "tui.theme": {"value": "tokyonight", "origin": {"layer": "default"}},
        "tui.theme_mode": {"value": "auto", "origin": {"layer": "default"}}
    })
}

#[test]
fn enter_opens_a_list_that_previews_as_it_moves_and_esc_puts_it_back() {
    // 2026-10-14 项目主人：明暗、配色按 Enter 弹列表选，和配置页选值的同一个组件、同一套键（上下、Enter 选、Esc 回），
    // 挪的时候当场换样子，Esc 换回原来的；不再左右切（以后主题多了、有别人做的，左右切不好用）。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    at_theme(&mut oobe, t0, &texts, defaults());
    assert_eq!(
        shown(&oobe),
        Some(("tokyonight".into(), Mode::Auto)),
        "没有头像：停在现在的配置"
    );
    // 左右不再换。
    oobe.key(key(KeyCode::Right), t0, &texts);
    assert_eq!(shown(&oobe), Some(("tokyonight".into(), Mode::Auto)));
    // 明暗那一格 Enter：开列表，停在「自动」；往下挪一个，当场换成深色；Enter 选好、关窗。
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let labels: Vec<String> = oobe
        .theme
        .picking
        .as_ref()
        .expect("开了列表")
        .choices
        .iter()
        .map(|c| c.label.clone())
        .collect();
    assert_eq!(labels, ["自动", "深色", "浅色"]);
    assert!(asks(&mut oobe).is_empty(), "开列表不写");
    oobe.key(key(KeyCode::Down), t0, &texts);
    assert_eq!(
        shown(&oobe),
        Some(("tokyonight".into(), Mode::Dark)),
        "挪到就换"
    );
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(oobe.theme.picking.is_none(), "选好关窗");
    assert_eq!(oobe.step, Step::Theme, "选好不往下走");
    // 配色那一格：挪到终端配色看一眼，Esc 换回 Tokyo Night，还在这一步。
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    assert_eq!(shown(&oobe), Some(("terminal".into(), Mode::Dark)));
    oobe.key(key(KeyCode::Esc), t0, &texts);
    assert!(oobe.theme.picking.is_none());
    assert_eq!(oobe.step, Step::Theme, "Esc 只关列表");
    assert_eq!(
        shown(&oobe),
        Some(("tokyonight".into(), Mode::Dark)),
        "换回原来的"
    );
    // 再开、选终端配色；「下一步」写两项。
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(key(KeyCode::Down), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(asks(&mut oobe).is_empty(), "选的时候不写");
    for _ in 0..3 {
        oobe.key(key(KeyCode::Down), t0, &texts);
    }
    assert_eq!(oobe.theme.slot, Slot::Next);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, params) = find(&asks(&mut oobe), "config.set");
    assert_eq!(
        params,
        json!({"changes": [
            {"key": "tui.theme_mode", "value": "dark"},
            {"key": "tui.theme", "value": "terminal"},
            {"key": "tui.theme_background", "value": true},
            {"key": "tui.mascot_home", "value": true},
            {"key": "tui.mascot_sidebar", "value": true}
        ]})
    );
    assert_eq!(oobe.step, Step::Theme, "写成了才往下走");
    oobe.answer(tag, Ok(json!({})), t0, &texts);
    assert_eq!(oobe.step, Step::Profile);
    assert_eq!(
        shown(&oobe),
        Some(("terminal".into(), Mode::Dark)),
        "写成了：引导关上以前一直照选的，不闪回原来的"
    );
}

#[test]
fn going_back_puts_the_old_look_back() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    at_theme(&mut oobe, t0, &texts, defaults());
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(key(KeyCode::Down), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(shown(&oobe).is_some());
    oobe.key(key(KeyCode::Esc), t0, &texts);
    assert_eq!(oobe.step, Step::Persona);
    assert_eq!(shown(&oobe), None, "退回去：照配置项，换回原来的样子");
}

#[test]
fn with_an_avatar_the_colors_start_on_follow_the_avatar_unless_already_chosen() {
    // 2026-10-14 项目主人定：人格那一步有头像（或写了主题色）的、配色没写过的，进来停在「跟随头像」。
    let texts = texts();
    let t0 = Instant::now();
    let dir = std::env::temp_dir().join(format!("miyu-theme-step-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("avatar.png");
    std::fs::write(&file, b"\x89PNG\r\n\x1a\n").unwrap();
    let mut oobe = fresh(t0);
    oobe.persona.avatar = Field::line().with(&file.display().to_string());
    at_theme(&mut oobe, t0, &texts, defaults());
    assert_eq!(oobe.theme.colors, 0, "停在跟随头像");
    assert_eq!(shown(&oobe).map(|p| p.0), Some("persona".into()));
    // 写过配色的照写的。
    let mut chosen = fresh(t0);
    chosen.persona.avatar = Field::line().with(&file.display().to_string());
    let items = json!({
        "tui.theme": {"value": "terminal", "origin": {"layer": "system"}},
        "tui.theme_mode": {"value": "light", "origin": {"layer": "system"}}
    });
    at_theme(&mut chosen, t0, &texts, items);
    assert_eq!(
        shown(&chosen),
        Some(("terminal".into(), Mode::Light)),
        "重走引导：照写过的停"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_refused_write_stays_and_says_why() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    at_theme(&mut oobe, t0, &texts, defaults());
    for _ in 0..4 {
        oobe.key(key(KeyCode::Down), t0, &texts);
    }
    assert_eq!(oobe.theme.slot, Slot::Next);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, _) = find(&asks(&mut oobe), "config.set");
    let refusal = crate::core::Refusal {
        reason: None,
        message: "写不进去".into(),
        data: serde_json::Value::Null,
    };
    oobe.answer(tag, Err(refusal), t0, &texts);
    assert_eq!(oobe.step, Step::Theme);
    assert_eq!(oobe.theme.error.as_deref(), Some("写不进去"));
}

#[test]
fn the_colors_list_offers_the_wallpaper_only_when_one_was_found() {
    // 2026-10-14 项目主人定：找不到壁纸的，配色里不列「跟随壁纸」。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    at_theme(&mut oobe, t0, &texts, defaults());
    oobe.key(key(KeyCode::Down), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let labels = |oobe: &Oobe| -> Vec<String> {
        let list = oobe.theme.picking.as_ref().expect("开了列表");
        list.choices.iter().map(|c| c.label.clone()).collect()
    };
    assert_eq!(labels(&oobe), ["跟随头像", "Tokyo Night", "终端配色"]);
    // 少了一行，挪到哪还是照值认：往上一个是跟随头像。
    oobe.key(key(KeyCode::Up), t0, &texts);
    assert_eq!(shown(&oobe).map(|p| p.0), Some("persona".into()));
    oobe.key(key(KeyCode::Esc), t0, &texts);
    // 找到了：列着，选上它当场换。
    oobe.theme.wallpaper = true;
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert_eq!(
        labels(&oobe),
        ["跟随头像", "跟随壁纸", "Tokyo Night", "终端配色"]
    );
    oobe.key(key(KeyCode::Up), t0, &texts);
    assert_eq!(shown(&oobe).map(|p| p.0), Some("wallpaper".into()));
}

#[test]
fn a_figure_row_writes_both_places_with_or_without_an_avatar() {
    // 2026-10-15 项目主人定：主题那一步「吉祥物」显示、不显示，一格同时管首页、侧边栏；默认停在显示。2026-10-29 项目主人：
    // 改叫「形象」，头像也归它管，人格有头像的照样列（原来不列）。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    at_theme(&mut oobe, t0, &texts, defaults());
    assert_eq!(
        oobe.theme.slots(),
        [
            Slot::Mode,
            Slot::Colors,
            Slot::Ground,
            Slot::Mascot,
            Slot::Next
        ]
    );
    assert!(!oobe.theme.mascot_off, "没写过的停在显示");
    for _ in 0..3 {
        oobe.key(key(KeyCode::Down), t0, &texts);
    }
    assert_eq!(oobe.theme.slot, Slot::Mascot);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let labels: Vec<String> = oobe
        .theme
        .picking
        .as_ref()
        .expect("开了列表")
        .choices
        .iter()
        .map(|c| c.label.clone())
        .collect();
    assert_eq!(labels, ["显示", "不显示"]);
    oobe.key(key(KeyCode::Down), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(oobe.theme.mascot_off);
    oobe.key(key(KeyCode::Down), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (_, params) = find(&asks(&mut oobe), "config.set");
    assert_eq!(
        params["changes"],
        json!([
            {"key": "tui.theme_mode", "value": "auto"},
            {"key": "tui.theme", "value": "tokyonight"},
            {"key": "tui.theme_background", "value": true},
            {"key": "tui.mascot_home", "value": false},
            {"key": "tui.mascot_sidebar", "value": false}
        ])
    );
    // 出厂关着、没人设过的：引导照旧停在显示（2026-10-26 项目主人：吉祥物默认关，引导里的照旧）。
    let mut shipped = fresh(t0);
    let mut items = defaults();
    items["tui.mascot_home"] = json!({"value": false, "origin": {"layer": "default"}});
    at_theme(&mut shipped, t0, &texts, items);
    assert!(!shipped.theme.mascot_off);
    // 写过不显示的：重走引导停在不显示。
    let mut again = fresh(t0);
    let mut items = defaults();
    items["tui.mascot_home"] = json!({"value": false, "origin": {"layer": "system"}});
    at_theme(&mut again, t0, &texts, items);
    assert!(again.theme.mascot_off);
    // 人格有头像：照样列这一格，选了不显示照样写那两项。
    let dir = std::env::temp_dir().join(format!("miyu-theme-mascot-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("avatar.png");
    std::fs::write(&file, b"\x89PNG\r\n\x1a\n").unwrap();
    let mut pictured = fresh(t0);
    pictured.persona.avatar = Field::line().with(&file.display().to_string());
    at_theme(&mut pictured, t0, &texts, defaults());
    assert!(pictured.theme.slots().contains(&Slot::Mascot), "有头像也列");
    pictured.theme.mascot_off = true;
    pictured.theme.slot = Slot::Next;
    pictured.key(key(KeyCode::Enter), t0, &texts);
    let (_, params) = find(&asks(&mut pictured), "config.set");
    assert!(
        params["changes"]
            .as_array()
            .is_some_and(|c| c.contains(&json!({"key": "tui.mascot_home", "value": false}))),
        "{params}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
