//! 从人格框、预设框按 `e` 进来改（蓝图 `tui.md`「新会话：人格、工作区」第 2b 条，2026-10-20 项目主人定）。

use serde_json::{Value, json};

use super::Item;
use crate::settings::{Settings, Texts};

fn texts() -> Texts {
    crate::config::Config::builtin().unwrap().text.settings
}

/// 从框进来的配置页。
fn lone() -> Settings {
    let mut page = Settings::default();
    page.watch.lone = true;
    page
}

/// 发了的最后一个请求的编号和方法。
fn last(page: &mut Settings) -> (u64, &'static str) {
    let (tag, method, _) = page.take_asks().pop().unwrap();
    (tag, method)
}

#[test]
fn words_deletes_and_preset_changes_are_remembered_once_saved() {
    let texts = texts();
    let mut page = lone();
    page.set_persona("mine", json!({"changes": []}), false);
    let (tag, _) = last(&mut page);
    page.answer(tag, Ok(json!({"persona": {"id": "mine"}})), &texts);
    page.delete_preset("full");
    let (tag, method) = last(&mut page);
    assert_eq!(method, "preset.delete");
    page.answer(tag, Ok(json!({})), &texts);
    assert_eq!(
        page.watch.saved,
        [
            (Item::Persona, "mine".to_string()),
            (Item::Preset, "full".to_string())
        ]
    );
}

#[test]
fn looks_only_refused_and_config_page_saves_are_not_remembered() {
    let texts = texts();
    // 只换头像：不进给模型的字。
    let mut page = lone();
    page.set_persona("mine", json!({"avatar": {"unset": true}}), false);
    let (tag, _) = last(&mut page);
    page.answer(tag, Ok(json!({"persona": {"id": "mine"}})), &texts);
    // 被拒的。
    page.set_persona("mine", json!({"changes": []}), false);
    let (tag, method) = last(&mut page);
    assert_eq!(method, "persona.set");
    let refused = crate::core::Refusal {
        message: "no".into(),
        reason: None,
        data: Value::Null,
    };
    page.answer(tag, Err(refused), &texts);
    assert!(page.watch.saved.is_empty());
    // 从 `/config` 进来改的（项目主人：配置页的不弹）。
    let mut page = Settings::default();
    page.set_persona("mine", json!({"changes": []}), false);
    let (tag, _) = last(&mut page);
    page.answer(tag, Ok(json!({"persona": {"id": "mine"}})), &texts);
    assert!(page.watch.saved.is_empty());
}

#[test]
fn the_box_page_goes_back_once_the_detail_closes() {
    let mut page = Settings::one(true, Item::Preset, "full".into());
    assert!(!page.lone_done(), "详情窗开着");
    assert!(page.take_asks().iter().any(|a| a.1 == "preset.get"));
    page.popup = None;
    assert!(page.lone_done());
    let page = Settings::default();
    assert!(!page.lone_done(), "从 /config 进来的不回框");
}
