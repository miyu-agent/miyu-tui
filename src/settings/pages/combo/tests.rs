//! 下拉加「其他」手写（蓝图「配置页」第 44 条）。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use super::{at, choices, other};
use crate::config::Config;
use crate::settings::pages::schema::Schema;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts};

fn texts() -> Texts {
    Config::builtin().unwrap().text.settings
}

/// 「用户」页只有「回复语言」那一项（核心 P-9 补）。
fn schema() -> Value {
    json!({"pages": [{"id": "user", "name": "用户"}], "groups": [{"id": "profile", "name": "关于你", "page": "user"}],
        "items": [{"key": "profile.preferred_reply_language", "type": "text", "control": "combo", "layers": ["system"],
            "suggestions": ["简体中文", "English", "日本語"], "name": "回复语言", "page": "user", "group": "profile"}]})
}

/// 系统配置里回复语言写的是 `value`（`None` 是没写）。
fn config(value: Option<&str>) -> Value {
    let layers: Vec<Value> = value
        .map(|v| json!({"origin": {"layer": "system", "line": 1}, "value": v, "used": true}))
        .into_iter()
        .collect();
    let value = value.map_or(Value::Null, |v| json!(v));
    json!({"items": {"profile.preferred_reply_language": {"value": value, "layers": layers}}})
}

#[test]
fn the_list_is_not_set_then_the_suggestions_then_other() {
    let texts = texts();
    let schema = Schema::read(&schema());
    let item = &schema.items[0];
    let list = choices(item, &texts);
    let labels: Vec<&str> = list.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["不设", "简体中文", "English", "日本語", "其他…"]);
    assert_eq!(list[0].value, Value::Null, "不设：去掉这一项");
    assert_eq!(at(item, &Value::Null), Value::Null, "没设的停在「不设」");
    assert_eq!(at(item, &json!("English")), json!("English"));
    assert_eq!(
        at(item, &json!("Klingon")),
        other(),
        "不在建议里的停在「其他…」"
    );
}

/// 开页、读回来，交回停在「用户」页的配置页。
fn opened(value: Option<&str>) -> Settings {
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "config.schema" => schema(),
            "config.get" => config(value),
            _ => json!({}),
        };
        page.answer(tag, Ok(got), &texts());
    }
    page
}

fn key(page: &mut Settings, code: KeyCode) {
    page.key(KeyEvent::new(code, KeyModifiers::NONE), &texts());
}

/// 这一次存的 `config.set` 的参数。
fn saved(page: &mut Settings) -> String {
    page.take_asks()
        .iter()
        .find(|(_, m, _)| *m == "config.set")
        .map(|(_, _, p)| p.to_string())
        .unwrap_or_default()
}

#[test]
fn other_opens_a_line_with_what_was_written_and_saves_it() {
    let texts = texts();
    let mut page = opened(Some("Klingon"));
    let item = page.more.schema.as_ref().unwrap().items[0].clone();
    page.open_combo(&item, &json!("Klingon"), &texts);
    let Some(Popup::Choose(choose)) = &page.popup else {
        panic!("开了选择窗：{:?}", page.popup);
    };
    assert_eq!(
        choose.choices[choose.sel].label, "其他…",
        "不在建议里的停在「其他…」"
    );
    key(&mut page, KeyCode::Enter);
    let Some(Popup::Line(edit)) = &page.popup else {
        panic!("「其他…」开编辑窗：{:?}", page.popup);
    };
    assert_eq!(edit.editor.text(), "Klingon", "原来写的留着");
    for _ in 0..7 {
        key(&mut page, KeyCode::Backspace);
    }
    for c in "粤语".chars() {
        key(&mut page, KeyCode::Char(c));
    }
    key(&mut page, KeyCode::Enter);
    let sent = saved(&mut page);
    assert!(
        sent.contains("profile.preferred_reply_language") && sent.contains("粤语"),
        "{sent}"
    );
}

#[test]
fn a_suggestion_is_saved_as_it_is_and_not_set_removes_it() {
    let texts = texts();
    let mut page = opened(Some("English"));
    let item = page.more.schema.as_ref().unwrap().items[0].clone();
    page.open_combo(&item, &json!("English"), &texts);
    key(&mut page, KeyCode::Char('k'));
    key(&mut page, KeyCode::Enter);
    let sent = saved(&mut page);
    assert!(sent.contains("简体中文"), "选了建议值就存它：{sent}");
    let mut page = opened(Some("English"));
    page.open_combo(&item, &json!("English"), &texts);
    for _ in 0..3 {
        key(&mut page, KeyCode::Char('k'));
    }
    key(&mut page, KeyCode::Enter);
    let sent = saved(&mut page);
    assert!(
        sent.contains("unset") || sent.contains("null"),
        "不设：去掉这一项：{sent}"
    );
}
