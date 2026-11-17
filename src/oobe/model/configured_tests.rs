//! 接模型那一步：配置里已经有、目录里没有的一家（扩展驱动接的这类）也认（「第一次打开的引导」第 16 条）。

use std::time::Instant;

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::Phase;
use super::rows::Row;
use crate::oobe::tests::{asks, find, fresh, key, texts, to_model};

#[test]
fn a_configured_provider_outside_the_catalog_is_listed_first_and_tested_as_it_is() {
    // 2026-11-17 实测报的：配置里只有一家扩展驱动接的供应商、没写 `models.chat`，引导「接模型」那一步列的只有目录里的
    // 常用几家，找不到它。接得上的排最前、标已配好，回车照配置里那一家试；核心报了毛病（接不上）的不列。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    to_model(&mut oobe, t0, &texts);
    let list = json!({"uses": {"chat": null}, "providers": [
        {"id": "antigravity", "name": {"value": "Antigravity"}, "driver": "antigravity",
         "models": [{"model": "gemini-3-pro", "ref": "antigravity/gemini-3-pro"}]},
        {"id": "broken", "driver": "gone", "problem": "unknown driver", "models": []}]});
    for (tag, method, _) in asks(&mut oobe) {
        let reply = match method {
            "model.list" => list.clone(),
            "provider.catalog" => json!({"providers": [
                {"id": "deepseek", "name": "DeepSeek", "supported": true}]}),
            "config.get" => json!({"items": {}}),
            _ => json!({}),
        };
        oobe.answer(tag, Ok(reply), t0, &texts);
    }
    assert!(matches!(oobe.model.phase, Phase::List));
    let Some(Row::Provider(first)) = oobe.model.rows.first() else {
        panic!("{:?}", oobe.model.rows)
    };
    assert_eq!(first.name, "Antigravity");
    assert_eq!(first.configured.as_deref(), Some("antigravity"));
    assert_eq!(oobe.model.cursor, 0, "光标停在它上面");
    let broken = |row: &Row| matches!(row, Row::Provider(p) if p.catalog == "broken");
    assert!(!oobe.model.rows.iter().any(broken), "接不上的不列");
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(matches!(oobe.model.phase, Phase::Testing(_)));
    let (_, params) = find(&asks(&mut oobe), "provider.test");
    assert_eq!(params, json!({"provider": "antigravity"}));
}
