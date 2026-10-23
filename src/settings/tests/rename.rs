//! 供应商改编号（「配置页」第 17 条，核心 8-34 `provider.rename`；2026-10-22 项目主人：「最好是能改，方便一些」）：窗里别的改动
//! 照旧编号先存，存成了再改编号；只改了编号的直接改。

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::{opened, press, texts};
use crate::core::Refusal;
use crate::settings::Settings;
use crate::settings::popup::Popup;

/// 停着的这一格清空了打上 `text`。
fn retype(page: &mut Settings, text: &str) {
    press(page, KeyCode::Enter);
    for _ in 0..40 {
        press(page, KeyCode::Backspace);
    }
    for c in text.chars() {
        press(page, KeyCode::Char(c));
    }
    press(page, KeyCode::Enter);
}

/// 进中转站（relay）的窗：先停在显示名称。
fn relay_form() -> Settings {
    let mut page = opened(false);
    press(&mut page, KeyCode::Down);
    press(&mut page, KeyCode::Enter);
    page
}

#[test]
fn other_changes_are_saved_first_then_the_core_renames_and_a_refusal_shows_in_the_window() {
    let mut page = relay_form();
    retype(&mut page, "Relay");
    press(&mut page, KeyCode::Up);
    retype(&mut page, "relay2");
    press(&mut page, KeyCode::Char('s'));
    let asks = page.take_asks();
    assert_eq!(asks.len(), 1, "{asks:?}");
    assert_eq!(asks[0].1, "config.set", "先照旧编号存别的");
    assert!(asks[0].2.to_string().contains("providers.relay.name"));
    page.answer(asks[0].0, Ok(json!({"keys": {}})), &texts());
    let asks = page.take_asks();
    let (tag, _, params) = asks
        .iter()
        .find(|(_, m, _)| *m == "provider.rename")
        .expect("存成了接着改编号");
    assert_eq!(params, &json!({"from": "relay", "to": "relay2"}));
    let refusal = Refusal {
        reason: Some("provider_exists".into()),
        message: "供应商编号已存在。".into(),
        data: json!(null),
    };
    page.answer(*tag, Err(refusal), &texts());
    let Some(Popup::Form(form)) = &page.popup else {
        panic!("没改成：窗留着");
    };
    assert_eq!(form.error.as_deref(), Some("供应商编号已存在。"));
}

#[test]
fn only_the_id_changed_renames_at_once_and_closes() {
    let mut page = relay_form();
    press(&mut page, KeyCode::Up);
    retype(&mut page, "relay2");
    press(&mut page, KeyCode::Char('s'));
    let asks = page.take_asks();
    assert_eq!(asks.len(), 1, "{asks:?}");
    assert_eq!(asks[0].1, "provider.rename", "没有别的改动：直接改编号");
    page.answer(asks[0].0, Ok(json!({})), &texts());
    assert!(page.popup.is_none(), "改成了关窗");
    assert!(page.draft.is_empty());
}
