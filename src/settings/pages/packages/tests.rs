//! 「软件包」先列包、进去一包一页（「配置页」第 43a 条，2026-10-16 项目主人定）。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use super::super::{Entry, Section};
use crate::config::Config;
use crate::settings::Settings;
use crate::settings::test_support::{config_all, model_list};

/// 两个有配置项的包、一个没有项的组：名字是核心照界面语言给的组名。
fn schema() -> Value {
    let item = |key: &str, name: &str, group: &str| {
        json!({"key": key, "control": "number", "layers": ["system", "personal"], "name": name,
               "page": "packages", "group": group})
    };
    json!({"pages": [{"id": "general", "name": "通用"}, {"id": "packages", "name": "软件包"}],
        "groups": [{"id": "memory", "name": "人格记忆", "page": "packages"},
                   {"id": "im", "name": "接入通讯平台", "page": "packages"},
                   {"id": "empty", "name": "空的", "page": "packages"}],
        "items": [item("memory.wait", "整理记忆前等多久", "memory"),
                  item("memory.rounds", "攒几轮再整理", "memory"),
                  item("im.listen", "端口", "im")]})
}

fn opened() -> (Settings, Config) {
    let config = Config::builtin().unwrap();
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "model.list" => model_list(),
            "config.get" => config_all(),
            "config.schema" => schema(),
            "package.list" => json!({"packages": [
                {"package": "memory", "name": "人格记忆", "summary": "让 AI 记住你"},
                {"package": "im", "name": "接入通讯平台", "summary": "在通讯平台里和 AI 聊天"}]}),
            _ => json!({"secrets": []}),
        };
        page.answer(tag, Ok(got), &config.text.settings);
    }
    (page, config)
}

fn press(page: &mut Settings, config: &Config, code: KeyCode) {
    page.key(
        KeyEvent::new(code, KeyModifiers::NONE),
        &config.text.settings,
    );
}

#[test]
fn packages_are_listed_first_and_each_opens_its_own_page() {
    let (mut page, config) = opened();
    let listed = page.more.packages();
    let rows: Vec<(&str, Option<&str>, usize)> = listed
        .iter()
        .map(|p| (p.name.as_str(), p.summary.as_deref(), p.count))
        .collect();
    assert_eq!(
        rows,
        [
            ("人格记忆", Some("让 AI 记住你"), 2),
            ("接入通讯平台", Some("在通讯平台里和 AI 聊天"), 1)
        ],
        "没有项的组不列"
    );
    for _ in 0..8 {
        if page.more.selected() == Entry::Page("packages".into()) {
            break;
        }
        press(&mut page, &config, KeyCode::Char('j'));
    }
    press(&mut page, &config, KeyCode::Enter);
    assert_eq!(page.more.section, Some(Section::Packages(0)), "先列包");
    press(&mut page, &config, KeyCode::Char('j'));
    press(&mut page, &config, KeyCode::Enter);
    assert_eq!(
        page.more.section,
        Some(Section::Page("packages/im".into(), 0)),
        "进去是这个包自己的一页"
    );
    let items = page.page_items("packages/im");
    assert_eq!(items.len(), 1, "只有这个包的项");
    press(&mut page, &config, KeyCode::Esc);
    assert_eq!(
        page.more.section,
        Some(Section::Packages(1)),
        "Esc 回到列表、停在它上面"
    );
    press(&mut page, &config, KeyCode::Esc);
    assert_eq!(page.more.section, None, "再 Esc 回主菜单");
}
