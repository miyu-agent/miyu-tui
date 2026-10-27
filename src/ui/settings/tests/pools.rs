//! 模型池那一页（「配置页」，2026-10-27 项目主人）：空池不写「空池」；「用于」放不下截掉，选中时成员栏下面写全。

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::{press, screen};
use crate::config::Config;
use crate::settings::Settings;

#[test]
fn a_pool_used_for_much_is_cut_in_its_row_and_listed_in_full_below_its_members() {
    let config = Config::builtin().unwrap();
    let mut list = crate::settings::test_support::model_list();
    list["pools"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name": "lite", "strategy": "pin", "models": [], "fixed": true}));
    let side = |id: &str, name: &str| json!({"id": id, "tier": "lite", "name": name, "summary": "", "resolved": "session"});
    list["side"] = json!([
        side("core/title", "起标题"),
        side("memory/organize", "整理记忆"),
        side("core/recap", "回顾这一轮做了什么"),
        side("im/glance", "看一眼群里在聊什么")
    ]);
    let mut p = Settings::open(false, true);
    for (tag, method, _) in p.take_asks() {
        let got = match method {
            "model.list" => list.clone(),
            "config.get" => crate::settings::test_support::config_all(),
            _ => json!({"secrets": []}),
        };
        p.answer(tag, Ok(got), &config.text.settings);
    }
    press(&mut p, &config, KeyCode::Enter);
    p.nav.page_step(true);
    p.nav.page_step(true);
    let (text, _) = screen(&mut p, &config, 120, 30);
    let row = text
        .lines()
        .find(|l| l.contains("轻量"))
        .unwrap_or_default()
        .to_string();
    let empty = text
        .lines()
        .find(|l| l.contains("empty"))
        .unwrap_or_default();
    assert!(!empty.trim_end().ends_with("空池"), "空池不写：{empty}");
    assert!(
        row.contains("用于：起标题") && row.contains('…'),
        "放不下截掉：{row}"
    );
    assert!(!row.contains("看一眼群里在聊什么"), "{row}");
    // 选中的就是轻量（固定的排最前、只有它）：成员栏下面写全。
    for name in [
        "用于",
        "起标题",
        "整理记忆",
        "回顾这一轮做了什么",
        "看一眼群里在聊什么",
    ] {
        assert!(
            text.lines().any(|l| l.trim_end().ends_with(name)),
            "成员栏下面写全「{name}」：\n{text}"
        );
    }
}
