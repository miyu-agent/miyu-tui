//! 软件包配置的几种写法（「配置页」第 33 条子规则，核心设计 53 第一档 PC-1）：小数开数的编辑窗、存小数；滑块在行上调，
//! `Enter` 开数的编辑窗（不再落到「这一项终端还不能改」）。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::json;

use crate::config::Config;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts};

fn texts() -> Texts {
    Config::builtin().unwrap().text.settings
}

/// 画图包两项：小数的温度、滑块的步数。
fn opened() -> Settings {
    let item = |key: &str, kind: &str, control: &str| {
        json!({"key": key, "type": kind, "control": control, "layers": ["system"], "name": key,
               "page": "packages", "group": "draw", "package": "draw", "min": 0, "max": 50, "step": 1})
    };
    let schema = json!({"pages": [{"id": "packages", "name": "软件包"}],
        "groups": [{"id": "draw", "name": "画图", "page": "packages"}],
        "items": [item("draw.temperature", "float", "number"), item("draw.steps", "int", "slider")]});
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "config.schema" => schema.clone(),
            "config.get" => json!({"items": {}}),
            _ => json!({}),
        };
        page.answer(tag, Ok(got), &texts());
    }
    page
}

#[test]
fn a_float_opens_the_number_editor_and_saves_a_float() {
    let mut page = opened();
    page.change("packages/draw", 0, &texts());
    let Some(Popup::Line(edit)) = &page.popup else {
        panic!("开数的编辑窗：{:?}", page.popup);
    };
    assert!(edit.number && edit.float);
    page.key(
        KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE),
        &texts(),
    );
    page.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &texts());
    let set = page
        .take_asks()
        .into_iter()
        .find(|(_, m, _)| *m == "config.set")
        .expect("存了");
    let change = &set.2["changes"][0];
    assert_eq!(change["key"], "draw.temperature");
    assert!(change["value"].is_f64(), "存成小数：{}", set.2);
    assert_eq!(change["value"], json!(1.0));
}

#[test]
fn enter_on_a_slider_opens_the_number_editor() {
    let mut page = opened();
    page.change("packages/draw", 1, &texts());
    let Some(Popup::Line(edit)) = &page.popup else {
        panic!("滑块上 Enter 开数的编辑窗打精确值：{:?}", page.popup);
    };
    assert!(edit.number && !edit.float);
}

fn press(page: &mut Settings, code: KeyCode) {
    page.key(KeyEvent::new(code, KeyModifiers::NONE), &texts());
}

/// 2026-10-31 项目主人定：滑块那一行 `←` `→` 直接调，不开窗；停手（按别的键）才存一次。
#[test]
fn arrows_move_a_slider_in_place_and_another_key_saves_once() {
    let mut page = opened();
    page.on_menu = false;
    page.more.section = Some(crate::settings::pages::Section::Page(
        "packages/draw".into(),
        1,
    ));
    page.take_asks();
    press(&mut page, KeyCode::Right);
    press(&mut page, KeyCode::Char('l'));
    let sliding = page.more.sliding.clone().expect("调着");
    assert_eq!(sliding.value, json!(2), "没值的从最小 0 起，两步");
    assert!(page.slide_due().is_some());
    assert!(page.popup.is_none(), "不开窗");
    assert!(
        !page.take_asks().iter().any(|(_, m, _)| *m == "config.set"),
        "调着不存"
    );
    assert_eq!(
        page.more.section,
        Some(crate::settings::pages::Section::Page(
            "packages/draw".into(),
            1
        )),
        "滑块那一行上 ← → 不退出"
    );
    press(&mut page, KeyCode::Up);
    let sets: Vec<_> = page
        .take_asks()
        .into_iter()
        .filter(|(_, m, _)| *m == "config.set")
        .collect();
    assert_eq!(sets.len(), 1, "按别的键先存，只存一次");
    assert_eq!(sets[0].2["changes"][0]["value"], json!(2));
    assert!(page.more.sliding.is_none());
    press(&mut page, KeyCode::Left);
    assert_eq!(
        page.more.section,
        Some(crate::settings::pages::Section::Store),
        "不是滑块的行上 ← 照旧退出（包自己那一页回「软件包」）"
    );
}
