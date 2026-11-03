//! 滑块那一行（「配置页」第 33 条子规则，2026-10-31 项目主人定）：16 格短条加数值，调着的照调着的画；光标停上去按键提示换成
//! 滑块的那一套。

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::{press, screen};
use crate::config::Config;
use crate::settings::Settings;
use crate::settings::pages::Section;

fn opened(config: &Config) -> Settings {
    let schema = json!({"pages": [{"id": "general", "name": "通用"}],
        "groups": [{"id": "draw", "name": "画图", "page": "general"}],
        "items": [{"key": "draw.temperature", "type": "float", "control": "slider", "min": 0, "max": 2,
                   "step": 0.1, "layers": ["system"], "name": "温度", "page": "general", "group": "draw"},
                  {"key": "draw.prompt", "type": "text", "control": "text", "layers": ["system"],
                   "name": "提示词", "page": "general", "group": "draw"}]});
    let data = json!({"items": {"draw.temperature": {"value": 1.0, "layers": [
        {"origin": {"layer": "system", "line": 1}, "value": 1.0, "used": true}]}}});
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "config.schema" => schema.clone(),
            "config.get" => data.clone(),
            _ => json!({}),
        };
        page.answer(tag, Ok(got), &config.text.settings);
    }
    page.on_menu = false;
    page.more.section = Some(Section::Page("general".into(), 0));
    page
}

#[test]
fn a_slider_row_is_a_short_track_and_its_value_and_moves_while_adjusted() {
    let config = Config::builtin().unwrap();
    let mut page = opened(&config);
    let (text, _) = screen(&mut page, &config, 100, 30);
    let row = text
        .lines()
        .find(|l| l.contains("温度"))
        .unwrap()
        .to_string();
    let track: String = row.chars().filter(|c| *c == '─' || *c == '●').collect();
    assert_eq!(track.chars().count(), 16, "16 格：{row}");
    assert_eq!(
        track.chars().position(|c| c == '●'),
        Some(8),
        "1.0 在 0～2 的正中：{row}"
    );
    assert!(row.trim_end().ends_with("1.0"), "{row}");
    assert!(
        text.contains("←/→ 调"),
        "光标在滑块上，提示换成滑块的：\n{text}"
    );
    press(&mut page, &config, KeyCode::Right);
    let (text, _) = screen(&mut page, &config, 100, 30);
    let row = text
        .lines()
        .find(|l| l.contains("温度"))
        .unwrap()
        .to_string();
    assert!(row.trim_end().ends_with("1.1"), "调着的当场画：{row}");
    press(&mut page, &config, KeyCode::Down);
    let (text, _) = screen(&mut page, &config, 100, 30);
    assert!(!text.contains("←/→ 调"), "挪开了提示换回去：\n{text}");
}
