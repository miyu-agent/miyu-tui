//! 存成了的回执：存的里头有要重启核心才生效的（核心 CPU-1 的 `core.workers`，`applies` 是 `restart`），回执说清楚。

use serde_json::json;

use crate::config::Config;
use crate::settings::{Settings, Texts};

fn texts() -> Texts {
    Config::builtin().unwrap().text.settings
}

/// 开一个只有一项的配置页，写那一项，核心照 `applies` 回。
fn written(applies: &str) -> Settings {
    let reply = json!({"keys": {"core.workers": {"value": 4, "effective": 4, "applies": applies}}});
    answered(reply)
}

/// 开一个只有一项的配置页，写那一项，核心照 `reply` 回。
fn answered(reply: serde_json::Value) -> Settings {
    let schema = json!({"pages": [{"id": "advanced", "name": "高级"}],
        "groups": [{"id": "core", "name": "核心", "page": "advanced"}],
        "items": [{"key": "core.workers", "type": "integer", "control": "number", "layers": ["system"],
            "name": "核心的工作线程数", "page": "advanced", "group": "core",
            "applies": reply["keys"]["core.workers"]["applies"]}]});
    let config = json!({"items": {"core.workers": {"value": 0}}});
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "config.schema" => schema.clone(),
            "config.get" => config.clone(),
            _ => json!({}),
        };
        page.answer(tag, Ok(got), &texts());
    }
    page.write_setting("core.workers", json!(4), &texts());
    let asks = page.take_asks();
    let (tag, _, _) = asks
        .iter()
        .find(|(_, m, _)| *m == "config.set")
        .expect("当场存");
    page.answer(*tag, Ok(reply), &texts());
    page
}

#[test]
fn a_setting_that_needs_a_restart_says_so_when_saved() {
    let page = written("restart");
    assert_eq!(
        page.status.as_ref().map(|(text, _)| text.as_str()),
        Some("重启核心后生效")
    );
    let page = written("next_turn");
    assert_eq!(
        page.status.as_ref().map(|(text, _)| text.as_str()),
        Some("已保存")
    );
}

#[test]
fn a_saved_setting_that_names_a_missing_persona_shows_the_core_warning() {
    // 核心 CK-1：指着现在没有的人格、预设照收，回应带 `problems`（已经照连接的语言说）：回执后面接上，黄字提示。
    let reply = json!({"keys": {"core.workers": {"value": 4, "effective": 4, "applies": "now"}},
        "problems": [{"code": "unknown_persona", "level": "warning", "key": "persona.default",
            "message": "没有人格 ghost"}]});
    let page = answered(reply);
    let (text, tone) = page.status.as_ref().expect("写了回执");
    assert_eq!(text, "已保存 · 没有人格 ghost");
    assert!(
        matches!(tone, crate::settings::Tone::Note),
        "提示，不是出错"
    );
}
