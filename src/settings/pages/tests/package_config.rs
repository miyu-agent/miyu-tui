//! 软件包配置的几种写法（「配置页」第 33 条子规则，核心设计 53 第一档 PC-1）：小数开数的编辑窗、存小数；滑块先开数的编辑窗，
//! 不再落到「这一项终端还不能改」。

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
fn a_slider_opens_the_number_editor_for_now() {
    let mut page = opened();
    page.change("packages/draw", 1, &texts());
    let Some(Popup::Line(edit)) = &page.popup else {
        panic!("滑块先当数填：{:?}", page.popup);
    };
    assert!(edit.number && !edit.float);
}
