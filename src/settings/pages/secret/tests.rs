//! 密钥项（蓝图「配置页」第 48 条）。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use crate::config::Config;
use crate::settings::pages::target::Target;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts};

fn texts() -> Texts {
    Config::builtin().unwrap().text.settings
}

/// 一个包一项密钥（只能写系统配置，同接入QQ 的令牌），系统配置里写的是 `value`。
fn opened(value: Value) -> Settings {
    let schema = json!({"pages": [{"id": "packages", "name": "软件包"}],
        "groups": [{"id": "miyu-onebot", "name": "接入QQ", "page": "packages"}],
        "items": [{"key": "miyu-onebot.token", "type": "secret", "control": "text",
            "layers": ["system"], "name": "令牌", "page": "packages", "group": "miyu-onebot"}]});
    let layers = if value.is_null() {
        json!([])
    } else {
        json!([{"origin": {"layer": "system", "line": 1}, "value": value, "used": true}])
    };
    let config = json!({"items": {"miyu-onebot.token": {"value": value, "layers": layers}}});
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "config.schema" => schema.clone(),
            "config.get" => config.clone(),
            _ => json!({}),
        };
        page.answer(tag, Ok(got), &texts());
    }
    page
}

fn item(page: &Settings) -> crate::settings::pages::schema::Item {
    page.more.schema.as_ref().unwrap().items[0].clone()
}

fn press(page: &mut Settings, code: KeyCode) {
    page.key(KeyEvent::new(code, KeyModifiers::NONE), &texts());
}

#[test]
fn the_row_only_says_whether_it_is_set() {
    let words = texts().more;
    let page = opened(Value::Null);
    let shown = crate::settings::pages::shown(&item(&page), &Value::Null, None, &words);
    assert_eq!(shown, words.secret[1], "没设写「未设置」");
    let set = json!({"secret": "miyu-onebot-token-abc"});
    let shown = crate::settings::pages::shown(&item(&page), &set, None, &words);
    assert_eq!(shown, words.secret[0], "设了写「已设置」，不露引用");
}

#[test]
fn enter_stores_the_secret_first_then_writes_the_reference() {
    let mut page = opened(json!({"secret": "old"}));
    let it = item(&page);
    page.open_secret(&it);
    let Some(Popup::Line(edit)) = &page.popup else {
        panic!("开编辑窗：{:?}", page.popup);
    };
    assert_eq!(edit.target, Target::Secret);
    assert_eq!(edit.editor.text(), "", "不露原来的引用");
    for c in "t0ken".chars() {
        press(&mut page, KeyCode::Char(c));
    }
    press(&mut page, KeyCode::Enter);
    assert!(page.popup.is_none(), "Enter 关窗");
    let asks = page.take_asks();
    let (tag, _, params) = asks
        .iter()
        .find(|(_, m, _)| *m == "secret.set")
        .expect("先存进密钥库");
    let name = params["name"].as_str().unwrap().to_string();
    assert!(
        name.starts_with("miyu-onebot-token-"),
        "照网页的起法：{name}"
    );
    assert_eq!(params["value"], "t0ken");
    assert!(
        asks.iter().all(|(_, m, _)| *m != "config.set"),
        "密钥存好以前不写配置"
    );
    page.answer(*tag, Ok(json!({})), &texts());
    let asks = page.take_asks();
    let (_, _, sent) = asks
        .iter()
        .find(|(_, m, _)| *m == "config.set")
        .expect("再写引用");
    assert_eq!(sent["layer"], "system", "只有系统配置这一层的写系统配置");
    assert_eq!(
        sent["changes"][0],
        json!({"key": "miyu-onebot.token", "value": {"secret": name},
               "expect": {"value": {"secret": "old"}}})
    );
    assert!(!sent.to_string().contains("t0ken"), "明文不进 config.set");
}

#[test]
fn an_empty_one_stores_nothing() {
    let mut page = opened(Value::Null);
    let it = item(&page);
    page.open_secret(&it);
    press(&mut page, KeyCode::Enter);
    assert!(page.popup.is_none());
    assert!(page.take_asks().is_empty(), "空的不存");
}
