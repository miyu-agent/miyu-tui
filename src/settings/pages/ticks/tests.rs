//! 勾选窗（蓝图「配置页」第 47a 条）。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use crate::config::Config;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts};

fn texts() -> Texts {
    Config::builtin().unwrap().text.settings
}

/// 隐私页一项带选项的列表（照核心 5-15 下的 `sandbox.private.remove`），个人设置里写的是 `value`；照 `Enter` 那条路开窗。
/// `ticks` 是核心给的画法提示（`"unlisted"` 反着勾），不写的照正着勾。
fn opened(value: Value) -> Settings {
    opened_with(value, Value::Null)
}

fn opened_with(value: Value, ticks: Value) -> Settings {
    let schema = json!({"pages": [{"id": "privacy", "name": "隐私与安全"}],
        "groups": [{"id": "private", "name": "私密目录", "page": "privacy"}],
        "items": [{"key": "sandbox.private.remove", "type": "list", "control": "list", "element": "text",
            "options": [{"value": "~/.ssh", "name": "~/.ssh"}, {"value": "~/.gnupg", "name": "~/.gnupg"},
                        {"value": "~/.aws", "name": "~/.aws"}],
            "layers": ["personal"], "name": "不当私密的出厂目录", "page": "privacy", "group": "private", "ticks": ticks}]});
    let layers = if value.is_null() {
        json!([])
    } else {
        json!([{"origin": {"layer": "personal", "line": 1}, "value": value, "used": true}])
    };
    let config = json!({"items": {"sandbox.private.remove": {"value": value, "layers": layers}}});
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "config.schema" => schema.clone(),
            "config.get" => config.clone(),
            _ => json!({}),
        };
        page.answer(tag, Ok(got), &texts());
    }
    page.change("privacy", 0, &texts());
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
    let mut page = opened(json!(["~/.aws"]));
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
        sent.contains(r#""value":["~/.ssh"]"#),
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
fn an_unlisted_list_ticks_what_is_not_in_the_value() {
    // 2026-10-21 项目主人：「默认私密目录」进去是几个已经勾着的目录，空格取消勾选、回车保存（核心给 `"ticks": "unlisted"`）。
    let mut page = opened_with(json!(["~/.gnupg"]), json!("unlisted"));
    assert_eq!(ticked(&page), [true, false, true], "不在值里的勾着");
    press(&mut page, KeyCode::Char(' '));
    press(&mut page, KeyCode::Enter);
    let sent = saved(&mut page);
    assert!(
        sent.contains(r#""value":["~/.ssh","~/.gnupg"]"#),
        "存的照旧是去掉的那几个（没勾的）：{sent}"
    );
}

#[test]
fn ctrl_a_ticks_all_or_clears_all() {
    // 2026-10-22 项目主人验收默认私密目录：「这里应该支持ctrl+A切换全选」。
    let ctrl_a = || KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
    let mut page = opened(json!(["~/.aws"]));
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
