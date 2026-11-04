//! 选 Miyu 自己的东西（「配置页」第 33 条子规则，核心设计 53 第三档 PC-3）：人格、知识库照列表开选择窗，模型开选默认模型的
//! 那个窗、选了写回这一项。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use crate::config::Config;
use crate::settings::popup::Popup;
use crate::settings::test_support::model_list;
use crate::settings::{Settings, Texts};

fn texts() -> Texts {
    Config::builtin().unwrap().text.settings
}

/// 画图包三项：画图用的人格、查资料的知识库、画图模型；另一项账号（记录列表），字段里有一个画图模型；再一项画图用的预设、
/// 一项轮着用的几个人格（排期第 41 条）。
fn opened(kb: Option<Value>) -> Settings {
    let item = |key: &str, kind: &str| {
        json!({"key": key, "type": kind, "control": "text", "layers": ["system"], "name": key,
               "page": "packages", "group": "draw", "package": "draw"})
    };
    let mut accounts = item("draw.accounts", "list");
    accounts["element"] = json!("record");
    accounts["control"] = json!("list");
    accounts["fields"] =
        json!([{"key": "model", "type": "reference", "control": "text", "name": "模型"}]);
    let mut rota = item("draw.rota", "list");
    rota["element"] = json!("persona");
    let schema = json!({"pages": [{"id": "packages", "name": "软件包"}],
        "groups": [{"id": "draw", "name": "画图", "page": "packages"}],
        "items": [item("draw.persona", "persona"), item("draw.kb", "knowledge"),
                  item("draw.model", "reference"), accounts, item("draw.preset", "preset"), rota]});
    let layer = |v: &str| {
        json!({"value": v, "layers": [
        {"origin": {"layer": "system", "line": 1}, "value": v, "used": true}]})
    };
    let config = json!({"items": {"draw.persona": layer("miyu"),
        "draw.model": layer("relay/zhipu/glm-5v"),
        "draw.preset": layer("full"),
        "draw.rota": {"value": ["dev", "gone"], "layers": [
            {"origin": {"layer": "system", "line": 2}, "value": ["dev", "gone"], "used": true}]}}});
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "config.schema" => Ok(schema.clone()),
            "config.get" => Ok(config.clone()),
            "model.list" => Ok(model_list()),
            "persona.list" => Ok(
                json!({"personas": [{"persona": "miyu", "name": "美羽"}, {"persona": "dev", "name": "开发"}]}),
            ),
            "preset.list" => Ok(
                json!({"presets": [{"preset": "full", "name": "全部"}, {"preset": null, "name": "无工具"}]}),
            ),
            "kb.list" => kb.clone().ok_or_else(|| crate::core::Refusal {
                reason: Some("unknown_method".into()),
                message: "no".into(),
                data: Value::Null,
            }),
            _ => Ok(json!({"secrets": []})),
        };
        page.answer(tag, got, &texts());
    }
    page
}

fn press(page: &mut Settings, code: KeyCode) {
    page.key(KeyEvent::new(code, KeyModifiers::NONE), &texts());
}

fn labels(page: &Settings) -> Vec<String> {
    match &page.popup {
        Some(Popup::Choose(c)) => c.choices.iter().map(|c| c.label.clone()).collect(),
        other => panic!("开选择窗：{other:?}"),
    }
}

#[test]
fn personas_and_knowledge_bases_are_picked_from_their_lists() {
    let kb = json!({"bases": [{"id": "notes", "name": "笔记"}, {"id": "docs"}]});
    let mut page = opened(Some(kb));
    assert_eq!(
        crate::settings::pages::shown(
            &page.more.schema.as_ref().unwrap().items[0],
            &json!("miyu"),
            page.more
                .named_item(&page.more.schema.as_ref().unwrap().items[0]),
            &texts().more
        ),
        "美羽",
        "那一行写名字"
    );
    page.change("packages/draw", 0, &texts());
    assert_eq!(labels(&page), ["不选", "美羽", "开发"], "第一行不选");
    page.popup = None;
    page.change("packages/draw", 1, &texts());
    assert_eq!(labels(&page), ["不选", "笔记", "docs"], "没名字的照编号");
}

#[test]
fn a_preset_item_is_picked_from_the_presets_and_none_unsets_it() {
    // 排期第 41 条（核心 e9c2b52cd）：`type = "preset"` 照 `preset.list` 选（「无工具」不是文件，不列），「不选」交 `null`。
    let mut page = opened(None);
    let item = page.more.schema.as_ref().unwrap().items[4].clone();
    assert_eq!(
        crate::settings::pages::shown(
            &item,
            &json!("full"),
            page.more.named_item(&item),
            &texts().more
        ),
        "全部"
    );
    assert_eq!(
        crate::settings::pages::shown(
            &item,
            &Value::Null,
            page.more.named_item(&item),
            &texts().more
        ),
        "不选",
        "没设的写不选"
    );
    page.change("packages/draw", 4, &texts());
    assert_eq!(labels(&page), ["不选", "全部"]);
    page.take_asks();
    press(&mut page, KeyCode::Up);
    press(&mut page, KeyCode::Enter);
    let set = page
        .take_asks()
        .into_iter()
        .find(|(_, m, _)| *m == "config.set")
        .expect("存了");
    let change = &set.2["changes"][0];
    assert_eq!(change["key"], "draw.preset");
    assert_eq!(change["value"], Value::Null, "不选：去掉这一项");
}

#[test]
fn a_list_of_personas_is_ticked_from_the_personas_and_keeps_ones_that_are_gone() {
    // 排期第 41 条：`type = "list"`、`element = "persona"` 照人格列表勾；写着、列表里没有了的接在后面勾着，不悄悄丢掉。
    let mut page = opened(None);
    let item = page.more.schema.as_ref().unwrap().items[5].clone();
    assert_eq!(
        crate::settings::pages::shown(
            &item,
            &json!(["dev", "gone"]),
            page.more.named_item(&item),
            &texts().more
        ),
        "开发、gone",
        "那一行写名字"
    );
    page.change("packages/draw", 5, &texts());
    let Some(Popup::Ticks(ticks)) = &page.popup else {
        panic!("开勾选窗：{:?}", page.popup);
    };
    let names: Vec<&str> = ticks.options.iter().map(|(_, n)| n.as_str()).collect();
    assert_eq!(names, ["美羽", "开发", "gone"]);
    assert_eq!(ticks.on, [false, true, true]);
    page.take_asks();
    press(&mut page, KeyCode::Char(' '));
    press(&mut page, KeyCode::Enter);
    let set = page
        .take_asks()
        .into_iter()
        .find(|(_, m, _)| *m == "config.set")
        .expect("存了");
    assert_eq!(set.2["changes"][0]["value"], json!(["miyu", "dev", "gone"]));
}

#[test]
fn without_a_knowledge_list_the_item_is_typed_in() {
    let mut page = opened(None);
    page.change("packages/draw", 1, &texts());
    assert!(
        matches!(page.popup, Some(Popup::Line(_))),
        "{:?}",
        page.popup
    );
}

#[test]
fn a_model_item_opens_the_model_picker_and_writes_back_to_itself() {
    let mut page = opened(None);
    page.change("packages/draw", 2, &texts());
    let Some(Popup::Pick(pick)) = &page.popup else {
        panic!("开选模型的窗：{:?}", page.popup);
    };
    assert!(pick.to.is_some());
    assert_eq!(
        pick.chosen(&page.view).as_deref(),
        Some("relay/zhipu/glm-5v"),
        "停在现在的值上"
    );
    page.take_asks();
    press(&mut page, KeyCode::Down);
    press(&mut page, KeyCode::Enter);
    let set = page
        .take_asks()
        .into_iter()
        .find(|(_, m, _)| *m == "config.set")
        .expect("存了");
    let change = &set.2["changes"][0];
    assert_eq!(
        change["key"], "draw.model",
        "写回这一项，不写默认用途：{}",
        set.2
    );
    assert!(change["value"].is_string());
    assert_ne!(change["value"], "relay/zhipu/glm-5v", "换成了挪到的那一个");
    page.change("packages/draw", 2, &texts());
    press(&mut page, KeyCode::Esc);
    assert!(page.popup.is_none());
    assert!(
        !page.take_asks().iter().any(|(_, m, _)| *m == "config.set"),
        "Esc 不存"
    );
}

#[test]
fn a_model_field_in_a_record_comes_back_to_the_form() {
    let mut page = opened(None);
    page.change("packages/draw", 3, &texts());
    press(&mut page, KeyCode::Char('a'));
    press(&mut page, KeyCode::Enter);
    assert!(matches!(page.popup, Some(Popup::Pick(_))));
    press(&mut page, KeyCode::Esc);
    assert!(matches!(page.popup, Some(Popup::Record(_))), "Esc 回表单窗");
    press(&mut page, KeyCode::Enter);
    press(&mut page, KeyCode::Enter);
    let Some(Popup::Record(form)) = &page.popup else {
        panic!("回到表单窗：{:?}", page.popup);
    };
    assert!(form.record.get("model").is_some_and(Value::is_string));
}
