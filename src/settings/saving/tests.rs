//! 存成了的回执：存的里头有要重启核心才生效的（核心 CPU-1 的 `core.workers`，`applies` 是 `restart`），回执说清楚。

use serde_json::json;

use crate::config::Config;
use crate::settings::{Settings, Texts};

fn texts() -> Texts {
    Config::builtin().unwrap().text.settings
}

/// 开一个只有一项的配置页，写那一项，交回那一次 `config.set` 的编号。
fn written(applies: &str) -> Settings {
    let schema = json!({"pages": [{"id": "advanced", "name": "高级"}],
        "groups": [{"id": "core", "name": "核心", "page": "advanced"}],
        "items": [{"key": "core.workers", "type": "integer", "control": "number", "layers": ["system"],
            "name": "核心的工作线程数", "page": "advanced", "group": "core", "applies": applies}]});
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
    let reply = json!({"keys": {"core.workers": {"value": 4, "effective": 4, "applies": applies}}});
    page.answer(*tag, Ok(reply), &texts());
    page
}

#[test]
fn a_setting_that_needs_a_restart_says_so_when_saved() {
    let page = written("restart");
    assert_eq!(
        page.status.as_ref().map(|(text, _)| text.as_str()),
        Some("已保存，重启核心后生效")
    );
    let page = written("next_turn");
    assert_eq!(
        page.status.as_ref().map(|(text, _)| text.as_str()),
        Some("已保存")
    );
}
