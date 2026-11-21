//! 供应商的窗只画这一家真有的项（核心 FX-11：`model.list` 的 `needs`、`package`；主人本机验收第 8 条：Antigravity 这种软件包带的
//! 供应商回车开窗，写着它没有的 Base URL、认证、API 密钥）。`none` 的那几行不画；包给的值暗着写、人照样能改；不动就存，配置
//! 一个字不改（网页同一天查出：原来不动就存会把 `driver`、`package` 写进配置，盖住包给的）。

use ratatui::crossterm::event::KeyCode;
use serde_json::{Value, json};

use super::{press, screen};
use crate::config::Config;
use crate::settings::Settings;

fn model_list() -> Value {
    json!({"uses": {"chat": null, "vision": null}, "pools": [], "providers": [
        {"id": "agy", "name": {"value": "Antigravity", "from": "package"}, "driver": "gravity/antigravity",
         "base_url": null, "needs": {"key": "none", "base_url": "none"}, "package": "gravity", "models": []},
        {"id": "relay", "name": {"value": "relay", "from": "id"}, "driver": "openai-chat",
         "base_url": "https://relay.example.invalid/v1", "key": {"ref": "secret:relay", "set": true},
         "needs": {"key": "required", "base_url": "required"}, "models": []}]})
}

fn config_all() -> Value {
    let packaged = |v: Value| json!({"origin": {"layer": "package", "package": "gravity"}, "value": v, "layers": []});
    json!({"items": {
        "providers.agy.name": packaged(json!("Antigravity")),
        "providers.agy.driver": packaged(json!("gravity/antigravity")),
        "providers.relay.driver": {"origin": {"layer": "system"}, "value": "openai-chat",
            "layers": [{"origin": {"layer": "system"}, "value": "openai-chat"}]},
        "providers.relay.base_url": {"origin": {"layer": "system"}, "value": "https://relay.example.invalid/v1",
            "layers": [{"origin": {"layer": "system"}, "value": "https://relay.example.invalid/v1"}]}
    }})
}

/// 读好了的配置页，光标在供应商栏第 `at` 家上，开它的窗。
fn opened(config: &Config, at: usize) -> Settings {
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "model.list" => model_list(),
            "config.get" => config_all(),
            "package.list" => {
                json!({"packages": [{"package": "gravity", "name": "Gravity", "enabled": true}]})
            }
            _ => json!({"secrets": []}),
        };
        page.answer(tag, Ok(got), &config.text.settings);
    }
    press(&mut page, config, KeyCode::Enter);
    for _ in 0..at {
        press(&mut page, config, KeyCode::Down);
    }
    press(&mut page, config, KeyCode::Enter);
    page
}

#[test]
fn a_package_provider_draws_only_the_fields_it_has() {
    let config = Config::builtin().unwrap();
    let mut page = opened(&config, 0);
    let text = screen(&mut page, &config, 120, 34).0;
    assert!(text.contains("编辑供应商 · Antigravity"), "{text}");
    for gone in ["Base URL", "认证", "API 密钥"] {
        assert!(!text.contains(gone), "这一家没有「{gone}」：\n{text}");
    }
    assert!(text.contains("显示名称") && text.contains("接口"), "{text}");
    // 普通的一家照旧都画。
    let mut page = opened(&config, 1);
    let text = screen(&mut page, &config, 120, 34).0;
    for kept in ["Base URL", "认证", "API 密钥"] {
        assert!(text.contains(kept), "普通的一家有「{kept}」：\n{text}");
    }
}

#[test]
fn saving_a_package_provider_untouched_writes_nothing() {
    let config = Config::builtin().unwrap();
    let mut page = opened(&config, 0);
    press(&mut page, &config, KeyCode::Char('s'));
    let writes: Vec<String> = page
        .take_asks()
        .into_iter()
        .filter(|(_, method, _)| matches!(*method, "config.set" | "secret.set"))
        .map(|(_, method, params)| format!("{method} {params}"))
        .collect();
    assert!(writes.is_empty(), "不动就存，配置一个字不改：{writes:?}");
}

#[test]
fn the_packages_driver_is_dim_until_the_person_changes_it() {
    let config = Config::builtin().unwrap();
    let texts = &config.text.settings;
    let page = opened(&config, 0);
    let Some(crate::settings::popup::Popup::Form(form)) = &page.popup else {
        panic!("开了窗：{:?}", page.popup);
    };
    let row = form
        .rows
        .iter()
        .find(|r| r.field == crate::settings::forms::Field::Driver)
        .expect("有接口那一行");
    let (spans, _) = super::super::fields::value(row, form, texts, false, 40);
    assert!(
        spans
            .iter()
            .any(|s| s.content.contains("gravity/antigravity"))
    );
    assert!(
        spans.iter().all(|s| s.style == crate::theme::dim()),
        "包给的接口暗着写：{spans:?}"
    );
}
