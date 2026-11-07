//! 供应商窗的驱动下拉照核心给的选项（核心 MD-1，排期第 62 条）：装着的扩展驱动列在内置的后面；配置里写了、不在选项里的照原值
//! 多列一项、停在它上面，不显示成「默认」。

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::super::{opened, press};
use crate::settings::forms::{Field, Val};
use crate::settings::popup::Popup;

#[test]
fn the_driver_list_follows_the_core_and_keeps_a_written_unknown_one() {
    let mut page = opened(false);
    page.more.raw.0 = Some(
        json!({"items": [{"key": "providers.<id>.driver", "type": "option", "options": [
        {"value": "openai-chat", "name": "openai-chat"},
        {"value": "anthropic", "name": "anthropic"},
        {"value": "openai-responses", "name": "openai-responses"},
        {"value": "acme/fast", "name": "acme/fast"}]}]}),
    );
    let data = page.data.clone().expect("读到了");
    page.draft
        .set("providers.relay.driver", json!("gone/drv"), &data);
    press(&mut page, KeyCode::Down);
    press(&mut page, KeyCode::Enter);
    let Some(Popup::Form(form)) = &page.popup else {
        panic!("开了中转站的窗");
    };
    let row = form
        .rows
        .iter()
        .find(|r| r.field == Field::Driver)
        .expect("有驱动那一行");
    let Val::Choice { options, at } = &row.val else {
        panic!("驱动是下拉");
    };
    assert_eq!(
        options,
        &[
            "",
            "openai-chat",
            "anthropic",
            "openai-responses",
            "acme/fast",
            "gone/drv"
        ]
    );
    assert_eq!(at.map(|i| options[i].as_str()), Some("gone/drv"));
}

#[test]
fn a_custom_provider_offers_the_extension_drivers_too() {
    let mut page = opened(false);
    page.more.raw.0 = Some(
        json!({"items": [{"key": "providers.<id>.driver", "type": "option", "options": [
        {"value": "openai-chat", "name": "openai-chat"},
        {"value": "acme/fast", "name": "acme/fast"}]}]}),
    );
    super::custom(&mut page);
    let Some(Popup::Form(form)) = &page.popup else {
        panic!("开了新供应商的窗");
    };
    let row = form
        .rows
        .iter()
        .find(|r| r.field == Field::Driver)
        .expect("有驱动那一行");
    let Val::Choice { options, at } = &row.val else {
        panic!("驱动是下拉");
    };
    assert_eq!(options, &["", "openai-chat", "acme/fast"]);
    assert_eq!(*at, Some(0), "新加的停在默认");
}
