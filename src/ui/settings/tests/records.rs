//! 记录列表的样子（「配置页」第 47b 条）：页里那一行用「、」连各条的标题；列表窗一条一行写标题；表单窗一个字段一行，密钥写
//! 设没设、滑块画短条。

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::{press, screen};
use crate::config::Config;
use crate::settings::Settings;
use crate::settings::pages::Section;

fn opened(config: &Config) -> Settings {
    let schema = json!({"pages": [{"id": "general", "name": "通用"}],
        "groups": [{"id": "draw", "name": "画图", "page": "general"}],
        "items": [{"key": "draw.accounts", "type": "list", "element": "record", "control": "list",
            "layers": ["system"], "name": "账号", "page": "general", "group": "draw", "title": "name",
            "fields": [
                {"key": "name", "type": "text", "control": "text", "name": "名字"},
                {"key": "key", "type": "secret", "control": "text", "name": "Key"},
                {"key": "limit", "type": "int", "control": "slider", "min": 0, "max": 50, "step": 5,
                 "name": "每日上限"}]}]});
    let value = json!([{"name": "主号", "key": {"secret": "k"}, "limit": 25}, {"limit": 5}]);
    let data = json!({"items": {"draw.accounts": {"value": value, "layers": [
        {"origin": {"layer": "system", "line": 1}, "value": value, "used": true}]}}});
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
fn the_row_lists_titles_the_list_window_too_and_the_form_shows_each_field() {
    let config = Config::builtin().unwrap();
    let mut page = opened(&config);
    let (text, _) = screen(&mut page, &config, 100, 30);
    let row = text
        .lines()
        .find(|l| l.contains("账号"))
        .unwrap()
        .to_string();
    assert!(row.contains("主号、第 2 项"), "{row}");
    press(&mut page, &config, KeyCode::Enter);
    let (text, _) = screen(&mut page, &config, 100, 30);
    assert!(text.contains("主号") && text.contains("第 2 项"), "{text}");
    assert!(text.contains("J/K 移动"), "{text}");
    press(&mut page, &config, KeyCode::Enter);
    let (text, _) = screen(&mut page, &config, 100, 30);
    assert!(text.contains("账号 › 主号"), "{text}");
    let key = text
        .lines()
        .find(|l| l.contains("Key"))
        .unwrap()
        .to_string();
    assert!(key.contains("已设置"), "只写设没设，不露引用：{key}");
    let limit = text
        .lines()
        .find(|l| l.contains("每日上限"))
        .unwrap()
        .to_string();
    assert!(
        limit.contains('●') && limit.trim_end().ends_with("25"),
        "{limit}"
    );
}

/// 表单里改密钥字段：编辑窗里打的字遮住（同第 48 条）。
#[test]
fn typing_a_record_secret_is_masked() {
    let config = Config::builtin().unwrap();
    let mut page = opened(&config);
    press(&mut page, &config, KeyCode::Enter);
    press(&mut page, &config, KeyCode::Enter);
    press(&mut page, &config, KeyCode::Down);
    press(&mut page, &config, KeyCode::Enter);
    for c in "zq".chars() {
        press(&mut page, &config, KeyCode::Char(c));
    }
    let (text, _) = screen(&mut page, &config, 100, 30);
    assert!(text.contains("••"), "{text}");
    assert!(!text.contains("zq"), "{text}");
}
