//! 勾选窗（蓝图「配置页」第 47a 条）。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use crate::config::Config;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts};

fn texts() -> Texts {
    Config::builtin().unwrap().text.settings
}

/// 一项带选项的列表（某个包的「要看的城市」），系统配置里写的是 `value`；照 `Enter` 那条路开窗。
fn opened(value: Value) -> Settings {
    let schema = json!({"pages": [{"id": "extras", "name": "扩展"}],
        "groups": [{"id": "weather", "name": "天气", "page": "extras"}],
        "items": [{"key": "weather.cities", "type": "list", "control": "list", "element": "text",
            "options": [{"value": "tokyo", "name": "东京"}, {"value": "osaka", "name": "大阪"},
                        {"value": "kyoto", "name": "京都"}],
            "layers": ["system"], "name": "要看的城市", "page": "extras", "group": "weather"}]});
    let layers = if value.is_null() {
        json!([])
    } else {
        json!([{"origin": {"layer": "system", "line": 1}, "value": value, "used": true}])
    };
    let config = json!({"items": {"weather.cities": {"value": value, "layers": layers}}});
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "config.schema" => schema.clone(),
            "config.get" => config.clone(),
            _ => json!({}),
        };
        page.answer(tag, Ok(got), &texts());
    }
    page.change("extras", 0, &texts());
    page
}

fn press(page: &mut Settings, code: KeyCode) {
    page.key(KeyEvent::new(code, KeyModifiers::NONE), &texts());
}

/// 勾着的几行。
fn ticked(page: &Settings) -> Vec<bool> {
    match &page.popup {
        Some(Popup::Ticks(edit)) => edit.on.clone(),
        other => panic!("勾选窗应开着：{other:?}"),
    }
}

/// 这一次存的 `config.set` 的参数；没存的是空的。
fn saved(page: &mut Settings) -> String {
    page.take_asks()
        .iter()
        .find(|(_, m, _)| *m == "config.set")
        .map(|(_, _, p)| p.to_string())
        .unwrap_or_default()
}

#[test]
fn a_list_with_options_ticks_them_and_enter_saves_in_option_order() {
    // 2026-10-21 项目主人：「做，照预设功能那样打勾」。
    let mut page = opened(json!(["kyoto"]));
    assert_eq!(ticked(&page), [false, false, true], "照现在的值勾上");
    press(&mut page, KeyCode::Char(' '));
    press(&mut page, KeyCode::Char('j'));
    press(&mut page, KeyCode::Char('j'));
    press(&mut page, KeyCode::Char(' '));
    assert_eq!(ticked(&page), [true, false, false]);
    assert_eq!(saved(&mut page), "", "勾的时候不存");
    press(&mut page, KeyCode::Enter);
    assert!(page.popup.is_none());
    let sent = saved(&mut page);
    assert!(
        sent.contains(r#""value":["tokyo"]"#),
        "Enter 收起时照选项的先后存：{sent}"
    );
}

#[test]
fn closing_without_a_change_saves_nothing() {
    let mut page = opened(Value::Null);
    assert_eq!(ticked(&page), [false, false, false]);
    press(&mut page, KeyCode::Down);
    press(&mut page, KeyCode::Esc);
    assert!(page.popup.is_none());
    assert_eq!(saved(&mut page), "");
}

#[test]
fn escape_closes_without_saving() {
    // 2026-10-21 项目主人：「空格取消勾选，回车保存」：Esc 不存。
    let mut page = opened(json!([]));
    press(&mut page, KeyCode::Char(' '));
    press(&mut page, KeyCode::Esc);
    assert!(page.popup.is_none());
    assert_eq!(saved(&mut page), "");
}

#[test]
fn ctrl_a_ticks_all_or_clears_all() {
    // 2026-10-22 项目主人验收时要的：「这里应该支持ctrl+A切换全选」。
    let ctrl_a = || KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
    let mut page = opened(json!(["kyoto"]));
    page.key(ctrl_a(), &texts());
    assert_eq!(ticked(&page), [true, true, true], "没全勾的全勾上");
    page.key(ctrl_a(), &texts());
    assert_eq!(ticked(&page), [false, false, false], "全勾着的全取消");
    press(&mut page, KeyCode::Char('a'));
    assert_eq!(ticked(&page), [false, false, false], "不按 Ctrl 的 a 不算");
    press(&mut page, KeyCode::Enter);
    let sent = saved(&mut page);
    assert!(sent.contains(r#""value":[]"#), "全取消照样存：{sent}");
}
