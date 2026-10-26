//! 填字的一项清空了存：去掉这一项，不写空字（2026-10-26 项目主人报：「用户名称」删空了存不进，核心回
//! 「profile.name 要写 最多 64 个字的文字，写的是 ""」）。

use serde_json::json;

use crate::config::Config;
use crate::settings::{Settings, Texts};

fn texts() -> Texts {
    Config::builtin().unwrap().text.settings
}

#[test]
fn a_cleared_text_item_is_removed_instead_of_written_empty() {
    let schema = json!({"pages": [{"id": "user", "name": "用户"}],
        "groups": [{"id": "profile", "name": "关于你", "page": "user"}],
        "items": [{"key": "profile.name", "type": "text", "control": "text",
            "layers": ["system", "personal"], "name": "AI 怎么称呼你", "page": "user", "group": "profile"}]});
    let written =
        json!([{"origin": {"layer": "personal", "line": 1}, "value": "小林", "used": true}]);
    let config = json!({"items": {"profile.name": {"value": "小林", "layers": written}}});
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "config.schema" => schema.clone(),
            "config.get" => config.clone(),
            _ => json!({}),
        };
        page.answer(tag, Ok(got), &texts());
    }
    page.write_setting("profile.name", json!(""), &texts());
    let asks = page.take_asks();
    let (_, _, sent) = asks
        .iter()
        .find(|(_, m, _)| *m == "config.set")
        .expect("当场存");
    assert_eq!(sent["layer"], "personal");
    assert_eq!(sent["changes"][0]["key"], "profile.name");
    assert_eq!(sent["changes"][0]["unset"], true, "去掉这一项：{sent}");
    assert!(
        sent["changes"][0].get("value").is_none(),
        "不写空字：{sent}"
    );
}
