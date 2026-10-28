//! 几行字的编辑窗（蓝图「配置页」第 45 条）。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use crate::config::Config;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts};

fn texts() -> Texts {
    Config::builtin().unwrap().text.settings
}

/// 「用户」页只有「更多描述」那一项（核心 P-9 三补），系统配置里写的是 `value`。
fn opened(value: Option<&str>) -> Settings {
    let schema = json!({"pages": [{"id": "user", "name": "用户"}],
        "groups": [{"id": "profile", "name": "关于你", "page": "user"}],
        "items": [{"key": "profile.about", "type": "text", "control": "textarea", "layers": ["system"],
            "name": "更多描述", "page": "user", "group": "profile"}]});
    let layers: Vec<Value> = value
        .map(|v| json!({"origin": {"layer": "system", "line": 1}, "value": v, "used": true}))
        .into_iter()
        .collect();
    let config = json!({"items": {"profile.about": {"value": value, "layers": layers}}});
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
    let now = value.map_or(Value::Null, |v| json!(v));
    page.open_textarea(&item, &now);
    page
}

fn key(page: &mut Settings, code: KeyCode, modifiers: KeyModifiers) {
    page.key(KeyEvent::new(code, modifiers), &texts());
}

fn type_text(page: &mut Settings, text: &str) {
    for c in text.chars() {
        key(page, KeyCode::Char(c), KeyModifiers::NONE);
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
fn enter_breaks_a_line_and_esc_saves_what_changed() {
    let mut page = opened(Some("写代码的"));
    key(&mut page, KeyCode::Enter, KeyModifiers::NONE);
    type_text(&mut page, "喜欢猫");
    let Some(Popup::Sheet(edit)) = &page.popup else {
        panic!("Enter 换行，窗还开着：{:?}", page.popup);
    };
    assert_eq!(edit.sheet.text(), "写代码的\n喜欢猫");
    key(&mut page, KeyCode::Esc, KeyModifiers::NONE);
    assert!(page.popup.is_none(), "Esc 收起");
    let sent = saved(&mut page);
    assert!(
        sent.contains("profile.about") && sent.contains("写代码的\\n喜欢猫"),
        "{sent}"
    );
}

#[test]
fn closing_unchanged_saves_nothing_and_clearing_removes_it() {
    let mut page = opened(Some("写代码的"));
    key(&mut page, KeyCode::Enter, KeyModifiers::NONE);
    key(&mut page, KeyCode::Char(' '), KeyModifiers::NONE);
    key(&mut page, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(
        saved(&mut page),
        "",
        "只在末尾多了空白、换行的不算改了，不存"
    );
    let mut page = opened(Some("写代码的"));
    for _ in 0..4 {
        key(&mut page, KeyCode::Backspace, KeyModifiers::NONE);
    }
    key(&mut page, KeyCode::Esc, KeyModifiers::NONE);
    let sent = saved(&mut page);
    assert!(
        sent.contains("unset") || sent.contains("null"),
        "清空了的去掉：{sent}"
    );
}

#[test]
fn ctrl_g_hands_the_text_out_and_comes_back_to_the_window() {
    let mut page = opened(Some("写代码的"));
    key(&mut page, KeyCode::Char('g'), KeyModifiers::CONTROL);
    assert_eq!(
        page.take_prompt().as_deref(),
        Some("写代码的"),
        "交给编辑器的是窗里的字"
    );
    page.prompt_edited(Some("编辑器里改的\n第二行\n".into()));
    let Some(Popup::Sheet(edit)) = &page.popup else {
        panic!("回到窗里：{:?}", page.popup);
    };
    assert_eq!(edit.sheet.text(), "编辑器里改的\n第二行", "末尾的换行去掉");
    key(&mut page, KeyCode::Esc, KeyModifiers::NONE);
    assert!(saved(&mut page).contains("编辑器里改的"), "收起时照常存");
}
