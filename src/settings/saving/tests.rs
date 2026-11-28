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

/// 开一个有两项（`core.workers`、`ui.compact`）的配置页，系统配置里 `core.workers` 写着 1。
fn two_items() -> Settings {
    let schema = json!({"pages": [{"id": "advanced", "name": "高级"}],
        "groups": [{"id": "core", "name": "核心", "page": "advanced"}],
        "items": [
            {"key": "core.workers", "type": "integer", "control": "number", "layers": ["system"],
                "name": "核心的工作线程数", "page": "advanced", "group": "core", "applies": "now"},
            {"key": "ui.compact", "type": "boolean", "control": "toggle", "layers": ["system"],
                "name": "紧凑", "page": "advanced", "group": "core", "applies": "now"}]});
    let config = json!({"items": {
        "core.workers": {"value": 1, "origin": {"layer": "system"},
            "layers": [{"origin": {"layer": "system"}, "value": 1}]},
        "ui.compact": {"value": false}}});
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

/// 交出去的 `config.set`：编号和每一项（键、值、`expect`）。
fn sets(page: &mut Settings) -> Vec<(u64, Vec<serde_json::Value>)> {
    page.take_asks()
        .into_iter()
        .filter(|(_, method, _)| *method == "config.set")
        .map(|(tag, _, params)| {
            (
                tag,
                params["changes"].as_array().cloned().unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn a_change_made_while_a_save_is_on_the_way_is_saved_after_it() {
    // 2026-11-28 审计第 3 条：保存在途时又改一项，原来 `save()` 直接返回不排队，回话一到草稿整份清掉、写「已保存」，第二项
    // 从没发给核心。
    let mut page = two_items();
    page.write_setting("core.workers", json!(4), &texts());
    let first = sets(&mut page);
    assert_eq!(first.len(), 1, "当场存");
    page.write_setting("ui.compact", json!(true), &texts());
    assert!(sets(&mut page).is_empty(), "在途的没回话，先不发");
    page.answer(first[0].0, Ok(json!({})), &texts());
    let second = sets(&mut page);
    assert_eq!(second.len(), 1, "回话以后接着存后改的那一项");
    assert_eq!(
        second[0].1,
        [json!({"key": "ui.compact", "value": true, "expect": {}})],
        "只发后改的"
    );
    assert_ne!(
        page.status.as_ref().map(|(text, _)| text.as_str()),
        Some("已保存"),
        "第二项还没存成"
    );
    page.answer(second[0].0, Ok(json!({})), &texts());
    assert!(page.draft.is_empty());
    assert_eq!(
        page.status.as_ref().map(|(text, _)| text.as_str()),
        Some("已保存")
    );
}

#[test]
fn changing_back_while_a_save_is_on_the_way_is_saved_too() {
    // 同上，第二种情形：在途时把刚改的改回原来的值，原来停在人不想要的那个值上。
    let mut page = two_items();
    page.write_setting("core.workers", json!(4), &texts());
    let first = sets(&mut page);
    page.write_setting("core.workers", json!(1), &texts());
    page.answer(first[0].0, Ok(json!({})), &texts());
    let second = sets(&mut page);
    assert_eq!(
        second.first().map(|(_, changes)| changes.clone()),
        Some(vec![
            json!({"key": "core.workers", "value": 1, "expect": {"value": 4}})
        ]),
        "照核心现在存着的 4 改回 1"
    );
}

#[test]
fn a_change_made_while_a_save_fails_is_still_saved() {
    // 同上，失败的那一支：原来 `failed()` 把后改的一起扔掉。没存成的那一项照旧扔掉。
    let mut page = two_items();
    page.write_setting("core.workers", json!(4), &texts());
    let first = sets(&mut page);
    page.write_setting("ui.compact", json!(true), &texts());
    let refusal = crate::core::Refusal {
        reason: None,
        message: "不行".into(),
        data: json!({}),
    };
    page.answer(first[0].0, Err(refusal), &texts());
    let second = sets(&mut page);
    assert_eq!(
        second.first().map(|(_, changes)| changes.clone()),
        Some(vec![
            json!({"key": "ui.compact", "value": true, "expect": {}})
        ]),
        "后改的照样存，没存成的那一项不再发"
    );
}
