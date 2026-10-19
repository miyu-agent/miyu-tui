//! 列表窗（蓝图「配置页」第 47 条）。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use crate::config::Config;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts};

fn texts() -> Texts {
    Config::builtin().unwrap().text.settings
}

/// 一个包一项列表（元素是 `element`），个人设置里写的是 `value`。
fn opened(element: &str, value: Value) -> Settings {
    let schema = json!({"pages": [{"id": "packages", "name": "软件包"}],
        "groups": [{"id": "im", "name": "接入通讯平台", "page": "packages"}],
        "items": [{"key": "im.whitelist", "type": "list", "control": "list", "element": element,
            "layers": ["personal"], "name": "群白名单", "page": "packages", "group": "im"}]});
    let layers = if value.is_null() {
        json!([])
    } else {
        json!([{"origin": {"layer": "personal", "line": 1}, "value": value, "used": true}])
    };
    let config = json!({"items": {"im.whitelist": {"value": value, "layers": layers}}});
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "config.schema" => schema.clone(),
            "config.get" => config.clone(),
            _ => json!({}),
        };
        page.answer(tag, Ok(got), &texts());
    }
    let item = page.more.schema.as_ref().unwrap().items[0].clone();
    page.open_list(&item, &value);
    page
}

fn press(page: &mut Settings, code: KeyCode) {
    page.key(KeyEvent::new(code, KeyModifiers::NONE), &texts());
}

fn type_text(page: &mut Settings, text: &str) {
    for c in text.chars() {
        press(page, KeyCode::Char(c));
    }
}

/// 列表窗里现在的几条。
fn items(page: &Settings) -> Vec<Value> {
    match &page.popup {
        Some(Popup::List(edit)) => edit.items.clone(),
        other => panic!("列表窗应开着：{other:?}"),
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
fn add_edit_delete_and_esc_saves_the_whole_list_once() {
    let mut page = opened("int", json!([111, 222]));
    assert_eq!(items(&page), [json!(111), json!(222)]);
    // a 新加一条：编辑窗，写好回到列表，停在新的那条。
    press(&mut page, KeyCode::Char('a'));
    assert!(matches!(page.popup, Some(Popup::Line(_))), "a 开编辑窗");
    type_text(&mut page, "333");
    press(&mut page, KeyCode::Enter);
    assert_eq!(items(&page), [json!(111), json!(222), json!(333)]);
    assert_eq!(saved(&mut page), "", "写好一条不存，Esc 收起时才存");
    // Enter 改第一条。
    press(&mut page, KeyCode::Char('k'));
    press(&mut page, KeyCode::Char('k'));
    press(&mut page, KeyCode::Enter);
    type_text(&mut page, "100");
    press(&mut page, KeyCode::Enter);
    // d 删第一条。
    press(&mut page, KeyCode::Char('d'));
    assert_eq!(items(&page), [json!(222), json!(333)]);
    press(&mut page, KeyCode::Esc);
    assert!(page.popup.is_none(), "Esc 收起");
    let sent = saved(&mut page);
    assert!(
        sent.contains("im.whitelist") && sent.contains("[222,333]"),
        "整份存成整数列表：{sent}"
    );
}

#[test]
fn whole_numbers_only_and_blank_removes() {
    let mut page = opened("int", json!([5]));
    press(&mut page, KeyCode::Char('a'));
    type_text(&mut page, "abc");
    press(&mut page, KeyCode::Enter);
    let Some(Popup::Line(edit)) = &page.popup else {
        panic!("写了别的编辑窗不关：{:?}", page.popup);
    };
    assert_eq!(
        edit.error.as_deref(),
        Some(texts().more.list.whole.as_str())
    );
    // 小数也不收。
    for _ in 0..3 {
        press(&mut page, KeyCode::Backspace);
    }
    type_text(&mut page, "1.5");
    press(&mut page, KeyCode::Enter);
    assert!(matches!(page.popup, Some(Popup::Line(_))), "小数不收");
    // Esc 不改，回到列表。
    press(&mut page, KeyCode::Esc);
    assert_eq!(items(&page), [json!(5)]);
    // 改成空的：去掉那一条。
    press(&mut page, KeyCode::Enter);
    press(&mut page, KeyCode::Backspace);
    press(&mut page, KeyCode::Enter);
    assert_eq!(items(&page), Vec::<Value>::new());
    press(&mut page, KeyCode::Esc);
    assert!(saved(&mut page).contains("[]"), "删光了写空列表");
}

#[test]
fn closing_untouched_saves_nothing_and_text_elements_stay_text() {
    let mut page = opened("text", Value::Null);
    assert_eq!(items(&page), Vec::<Value>::new());
    press(&mut page, KeyCode::Esc);
    assert!(page.popup.is_none());
    assert_eq!(saved(&mut page), "", "没动过不存");
    let mut page = opened("text", json!(["早"]));
    press(&mut page, KeyCode::Char('a'));
    type_text(&mut page, "42");
    press(&mut page, KeyCode::Enter);
    assert_eq!(items(&page), [json!("早"), json!("42")], "字的列表照字存");
}

#[test]
fn an_empty_list_reads_like_an_unset_one() {
    let words = texts().more;
    let page = opened("text", json!([]));
    let item = page.more.schema.as_ref().unwrap().items[0].clone();
    assert_eq!(
        crate::settings::pages::shown(&item, &json!([]), None, &words),
        "—",
        "删光了的那一行不是空白"
    );
    assert_eq!(
        crate::settings::pages::shown(&item, &json!(["早", "晚"]), None, &words),
        "早、晚"
    );
}
