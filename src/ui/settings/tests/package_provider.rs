//! 软件包带的供应商（核心 PV-4）：`config.get` 里这一家几项的来源是 `{"layer": "package", "package": "<包>"}`，供应商的窗里
//! 右边写「来自软件包 <包的名字>」（蓝图「配置页」第 17 条）；接口照包给的那一个显示，不写成「默认」。

use serde_json::{Value, json};

use super::{press, screen};
use crate::config::Config;
use crate::settings::Settings;
use ratatui::crossterm::event::KeyCode;

fn model_list() -> Value {
    json!({"uses": {"chat": null, "vision": null}, "pools": [],
        "providers": [{"id": "acme", "name": {"value": "Acme 云", "from": "package"}, "driver": "anthropic",
            "base_url": "https://acme.example.invalid/v1", "models": []}]})
}

fn config_all() -> Value {
    let packaged = |v: Value| json!({"origin": {"layer": "package", "package": "acme-suite"}, "value": v, "layers": []});
    json!({"items": {
        "providers.acme.name": packaged(json!("Acme 云")),
        "providers.acme.base_url": packaged(json!("https://acme.example.invalid/v1")),
        "providers.acme.driver": packaged(json!("anthropic"))
    }})
}

fn page(config: &Config, packages: Value) -> Settings {
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "model.list" => model_list(),
            "config.get" => config_all(),
            "package.list" => packages.clone(),
            _ => json!({"secrets": []}),
        };
        page.answer(tag, Ok(got), &config.text.settings);
    }
    page
}

/// 开那一家的窗，交回画出来的字。
fn form(packages: Value) -> String {
    let config = Config::builtin().unwrap();
    let mut p = page(&config, packages);
    press(&mut p, &config, KeyCode::Enter);
    press(&mut p, &config, KeyCode::Enter);
    screen(&mut p, &config, 120, 34).0
}

#[test]
fn values_a_package_gives_say_which_package_they_come_from() {
    let text = form(
        json!({"packages": [{"package": "acme-suite", "name": "Acme 全家桶", "enabled": true}]}),
    );
    assert!(text.contains("编辑供应商 · Acme 云"), "{text}");
    let line = |label: &str| {
        text.lines()
            .find(|l| l.contains(label))
            .unwrap_or_else(|| panic!("缺「{label}」：\n{text}"))
            .to_string()
    };
    for label in ["显示名称", "Base URL"] {
        assert!(
            line(label).trim_end().ends_with("来自软件包 Acme 全家桶"),
            "{label}：\n{text}"
        );
    }
    let driver = line("接口");
    assert!(driver.contains("anthropic"), "接口照包给的显示：\n{text}");
    assert!(
        driver.trim_end().ends_with("来自软件包 Acme 全家桶"),
        "{text}"
    );
    assert!(!line("认证").contains("来自软件包"), "包不给 key：\n{text}");
}

#[test]
fn a_package_missing_from_the_list_is_named_by_its_id() {
    let text = form(json!({"packages": []}));
    assert!(text.contains("来自软件包 acme-suite"), "{text}");
}
