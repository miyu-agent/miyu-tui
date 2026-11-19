//! 选项的说明和提示（核心设计 72，蓝图「配置页」第 33 条「选项的说明和提示」）：`description` 暗色写在名字下面、折行写全，
//! 核心的 `note` 照旧接在名字后面；带 `notice` 的选项选了照常存，同时弹一个只有「知道了」的提示窗。不认键名、不认值。

use ratatui::crossterm::event::KeyCode;
use serde_json::{Value, json};

use super::{press, screen};
use crate::config::Config;
use crate::settings::Settings;

const LONG: &str = "对话模型是 DeepSeek 官方、使用 Responses 接口时，由模型自己搜索网页、读取链接。搜索更深入，但每次搜索会多用约一万个 token。";
const NOTICE: &str = "这个搜索服务只适用于 DeepSeek 官方的 Responses 接口";

fn schema() -> Value {
    json!({"pages": [{"id": "general", "name": "通用"}],
        "groups": [{"id": "net", "name": "网络搜索", "page": "general"}],
        "items": [{"key": "net.search.source", "type": "option", "control": "select", "layers": ["system"],
            "name": "联网服务", "page": "general", "group": "net", "default": "auto",
            "options": [{"value": "auto", "name": "自动"},
                        {"value": "deepseek", "name": "DeepSeek 搜索接口", "description": "按次计费。", "note": "需要密钥"},
                        {"value": "deepseek-responses", "name": "DeepSeek Responses 接口内置",
                         "description": LONG, "notice": NOTICE}]}]})
}

/// 开着配置页、进「通用」、在「联网服务」上按 `Enter` 开选择窗。
fn opened(config: &Config) -> Settings {
    let texts = &config.text.settings;
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "config.schema" => schema(),
            "config.get" => json!({"items": {}}),
            _ => json!({}),
        };
        page.answer(tag, Ok(got), texts);
    }
    press(&mut page, config, KeyCode::Enter);
    press(&mut page, config, KeyCode::Enter);
    page
}

/// 发出去的 `config.set` 的参数（没发的是 `None`）。
fn sent(page: &mut Settings) -> Option<String> {
    page.take_asks()
        .into_iter()
        .find(|(_, m, _)| *m == "config.set")
        .map(|(_, _, params)| params.to_string())
}

#[test]
fn a_description_goes_under_the_name_and_a_core_note_stays_after_it() {
    let config = Config::builtin().unwrap();
    let mut page = opened(&config);
    let (text, _) = screen(&mut page, &config, 100, 34);
    let lines: Vec<&str> = text.lines().collect();
    let at = |word: &str| {
        lines
            .iter()
            .position(|l| l.contains(word))
            .unwrap_or_else(|| panic!("没有「{word}」：\n{text}"))
    };
    let search = at("DeepSeek 搜索接口");
    assert!(
        lines[search].contains("需要密钥"),
        "note 接在名字后面：\n{text}"
    );
    assert!(
        !lines[search].contains("按次计费"),
        "说明不挤在名字后面：\n{text}"
    );
    assert!(
        lines[search + 1].contains("按次计费。"),
        "说明写在名字下面：\n{text}"
    );
    // 长的说明折行写全，一字不少，和名字左对齐。
    let built = at("DeepSeek Responses 接口内置");
    let name_x = lines[built].find("DeepSeek").unwrap();
    let joined: String = lines[built + 1..]
        .iter()
        .take_while(|l| !l.trim().is_empty() && !l.contains('['))
        .map(|l| {
            assert_eq!(
                l.find(|c: char| c != ' ').unwrap_or(0),
                name_x,
                "和名字左对齐：\n{text}"
            );
            l.trim()
        })
        .collect();
    assert_eq!(joined.replace(' ', ""), LONG.replace(' ', ""), "\n{text}");
    assert!(lines[built + 2].contains("token"), "放不下就折行：\n{text}");
}

#[test]
fn picking_an_option_with_a_notice_saves_it_and_shows_the_notice_once() {
    let config = Config::builtin().unwrap();
    let mut page = opened(&config);
    sent(&mut page);
    press(&mut page, &config, KeyCode::Down);
    press(&mut page, &config, KeyCode::Down);
    press(&mut page, &config, KeyCode::Enter);
    let set = sent(&mut page).expect("照常存，不等提示窗");
    assert!(set.contains("deepseek-responses"), "{set}");
    let (text, _) = screen(&mut page, &config, 100, 34);
    assert!(text.contains(NOTICE), "弹出那一句：\n{text}");
    assert!(text.contains("[ 知道了 ]"), "只有一个按钮：\n{text}");
    assert!(!text.contains("[ 取消 ]"), "\n{text}");
    press(&mut page, &config, KeyCode::Enter);
    let (text, _) = screen(&mut page, &config, 100, 34);
    assert!(!text.contains(NOTICE), "按了就关：\n{text}");
    assert!(sent(&mut page).is_none(), "关提示窗不再存一次");
}

#[test]
fn an_option_without_a_notice_opens_nothing_after_saving() {
    let config = Config::builtin().unwrap();
    let mut page = opened(&config);
    sent(&mut page);
    press(&mut page, &config, KeyCode::Down);
    press(&mut page, &config, KeyCode::Enter);
    assert!(sent(&mut page).is_some_and(|s| s.contains("\"deepseek\"")));
    let (text, _) = screen(&mut page, &config, 100, 34);
    assert!(!text.contains("知道了"), "没有 notice 的不弹：\n{text}");
}
