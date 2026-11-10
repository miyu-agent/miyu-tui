//! 选项由包在运行时给的文字项（核心 SG-2 的 `choices_from`）：问到了开选择窗，问不到退回编辑窗。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use super::read;
use crate::config::Config;
use crate::core::Refusal;
use crate::settings::Settings;
use crate::settings::popup::Popup;
use crate::settings::test_support::{config_all, model_list};

/// 语音包的两项：音色的选项由包给，另一项是普通的文字框。
fn schema() -> Value {
    json!({"pages": [{"id": "general", "name": "通用"}, {"id": "packages", "name": "软件包"}],
        "groups": [{"id": "say", "name": "朗读", "page": "packages"}],
        "items": [
            {"key": "say.voice", "control": "text", "type": "text", "layers": ["system"], "name": "音色",
             "page": "packages", "group": "say", "package": "say",
             "choices_from": {"package": "say", "method": "voices"}},
            {"key": "say.note", "control": "text", "type": "text", "layers": ["system"], "name": "备注",
             "page": "packages", "group": "say", "package": "say"}]})
}

/// 开着配置页，系统配置里音色写的是 `stored`；交回页面和它发出去还没答的请求。
fn opened(stored: Option<&str>) -> (Settings, Config) {
    let config = Config::builtin().unwrap();
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "model.list" => model_list(),
            "config.get" => {
                let mut all = config_all();
                if let Some(stored) = stored {
                    let origin = json!({"layer": "system", "line": 1});
                    all["items"]["say.voice"] = json!({"value": stored, "origin": origin, "layers": [
                        {"origin": origin, "value": stored, "used": true}]});
                }
                all
            }
            "config.schema" => schema(),
            "package.list" => json!({"packages": [{"package": "say", "name": "朗读"}]}),
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

/// 进语音包那一页，光标停在第 `at` 项上按 `Enter`；交回这一下发出去的请求。
fn enter_item(page: &mut Settings, config: &Config, at: usize) -> Vec<(u64, &'static str, Value)> {
    if page.more.section.is_none() {
        for _ in 0..12 {
            if page.more.selected() == crate::settings::pages::Entry::Packages {
                break;
            }
            press(page, config, KeyCode::Char('j'));
        }
        press(page, config, KeyCode::Enter);
        press(page, config, KeyCode::Char('.'));
        press(page, config, KeyCode::Enter);
        page.take_asks();
    }
    let here = crate::settings::pages::Section::Page("packages/say".into(), at);
    if page.more.section != Some(here.clone()) {
        assert!(
            matches!(
                page.more.section,
                Some(crate::settings::pages::Section::Page(..))
            ),
            "进了包那一页：{:?}",
            page.more.section
        );
        page.more.section = Some(here);
    }
    press(page, config, KeyCode::Enter);
    page.take_asks()
}

fn refused() -> Refusal {
    Refusal {
        reason: Some("program_not_running".into()),
        message: "方法执行失败。".into(),
        data: json!({"reason": "program_not_running"}),
    }
}

#[test]
fn enter_asks_the_package_and_its_choices_open_the_chooser() {
    let (mut page, config) = opened(Some("b"));
    let asks = enter_item(&mut page, &config, 0);
    assert_eq!(asks.len(), 1, "{asks:?}");
    let (tag, method, params) = &asks[0];
    assert_eq!(*method, "package.call");
    assert_eq!(
        *params,
        json!({"package": "say", "method": "voices"}),
        "不带 params"
    );
    assert!(page.popup.is_none(), "答回来以前不开窗");
    // 问着的时候再按一次不重复问。
    assert!(enter_item(&mut page, &config, 0).is_empty());
    let got = json!({"choices": [
        {"value": "a", "name": "温柔", "description": "女声"},
        {"value": "b"},
        {"name": "没有值的不要"}]});
    page.answer(*tag, Ok(got), &config.text.settings);
    let Some(Popup::Choose(choose)) = &page.popup else {
        panic!("开选择窗：{:?}", page.popup);
    };
    assert_eq!(
        (choose.key.as_str(), choose.title.as_str()),
        ("say.voice", "音色")
    );
    let rows: Vec<(&str, Option<&str>)> = choose
        .choices
        .iter()
        .map(|c| (c.label.as_str(), c.note.as_deref()))
        .collect();
    assert_eq!(
        rows,
        [("温柔", Some("女声")), ("b", None)],
        "名字没写的照值写"
    );
    assert_eq!(choose.sel, 1, "停在存着的值上");
    // 选了第一行：把 value 写进配置。
    press(&mut page, &config, KeyCode::Char('k'));
    press(&mut page, &config, KeyCode::Enter);
    let saved = page.take_asks();
    let set = saved
        .iter()
        .find(|(_, method, _)| *method == "config.set")
        .unwrap_or_else(|| panic!("{saved:?}"));
    assert!(set.2.to_string().contains("\"a\""), "{}", set.2);
}

#[test]
fn a_refusal_or_an_empty_list_falls_back_to_the_text_box() {
    for answer in [
        Err(refused()),
        Ok(json!({"choices": []})),
        Ok(json!({"choices": "不是列表"})),
        Ok(json!({})),
    ] {
        let (mut page, config) = opened(Some("手写的"));
        let asks = enter_item(&mut page, &config, 0);
        page.answer(asks[0].0, answer.clone(), &config.text.settings);
        let Some(Popup::Line(edit)) = &page.popup else {
            panic!("退回编辑窗：{answer:?} → {:?}", page.popup);
        };
        assert_eq!(edit.key, "say.voice");
        assert_eq!(edit.editor.text(), "手写的", "原来写的还在");
    }
}

#[test]
fn a_stored_value_that_is_not_listed_stays_as_a_row() {
    let got = json!({"choices": [{"value": "a", "name": "温柔"}]});
    let list = read(&got, &json!("旧的")).unwrap();
    let rows: Vec<&str> = list.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(rows, ["温柔", "旧的"]);
    assert_eq!(list[1].value, json!("旧的"), "选它照旧留着");
    // 在列表里的、没设的不另补。
    assert_eq!(read(&got, &json!("a")).unwrap().len(), 1);
    assert_eq!(read(&got, &Value::Null).unwrap().len(), 1);
    let (mut page, config) = opened(Some("旧的"));
    let asks = enter_item(&mut page, &config, 0);
    page.answer(asks[0].0, Ok(got), &config.text.settings);
    let Some(Popup::Choose(choose)) = &page.popup else {
        panic!("{:?}", page.popup);
    };
    assert_eq!(choose.sel, 1, "停在存着的那一行");
}

#[test]
fn an_item_without_choices_from_opens_the_text_box_at_once() {
    let (mut page, config) = opened(None);
    let asks = enter_item(&mut page, &config, 1);
    assert!(asks.is_empty(), "不问包：{asks:?}");
    assert!(
        matches!(page.popup, Some(Popup::Line(_))),
        "{:?}",
        page.popup
    );
}
