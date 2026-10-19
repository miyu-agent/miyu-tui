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

#[test]
fn a_package_with_two_groups_is_one_row_and_its_page_titles_the_second_group() {
    // 2026-10-19 核心把联网搜索的 key、地址分出「API Key 和地址」一组（项都还带 `package: "net"`，项目主人要标题和空行）。
    let config = Config::builtin().unwrap();
    let item = |key: &str, name: &str, group: &str| {
        json!({"key": key, "control": "text", "layers": ["personal"], "name": name,
               "page": "packages", "group": group, "package": "net"})
    };
    let schema = json!({"pages": [{"id": "packages", "name": "软件包"}],
        "groups": [{"id": "net", "name": "联网搜索", "page": "packages"},
                   {"id": "net_keys", "name": "API Key 和地址", "page": "packages"}],
        "items": [item("net.search.source", "搜索服务供应商", "net"),
                  item("net.search.tavily.key", "Tavily", "net_keys"),
                  item("net.search.searxng.url", "SearXNG 的地址", "net_keys")]});
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "config.schema" => schema.clone(),
            "config.get" => json!({"items": {}}),
            _ => json!({}),
        };
        page.answer(tag, Ok(got), &config.text.settings);
    }
    let listed = page.more.packages();
    let rows: Vec<(&str, &str, usize)> = listed
        .iter()
        .map(|p| (p.id.as_str(), p.name.as_str(), p.count))
        .collect();
    assert_eq!(
        rows,
        [("net", "联网搜索", 3)],
        "合成一行，不把第二组当成一个包"
    );
    let names: Vec<String> = page
        .more
        .rows("packages/net")
        .into_iter()
        .map(|r| {
            let s = page.more.schema.as_ref().unwrap();
            match r {
                super::super::schema::Row::Group(g) => format!("# {}", s.groups[g].name),
                super::super::schema::Row::Item(i) => s.items[i].name.clone(),
            }
        })
        .collect();
    assert_eq!(
        names,
        [
            "搜索服务供应商",
            "# API Key 和地址",
            "Tavily",
            "SearXNG 的地址"
        ],
        "和包同编号的那一组不写组名，第二组写"
    );
}
