//! 改正在用的人格、预设（蓝图 `tui.md`「新会话：人格、工作区」第 2b 条，2026-10-20 项目主人定）。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use super::Item;
use crate::settings::pages::choose::{LineEdit, PersonaView};
use crate::settings::pages::persona_edit::PromptEdit;
use crate::settings::pages::target::Target;
use crate::settings::popup::{Delete, Popup};
use crate::settings::{Settings, Texts};

fn texts() -> Texts {
    crate::config::Config::builtin().unwrap().text.settings
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn view() -> PersonaView {
    PersonaView {
        id: "mine".into(),
        detail: None,
        error: None,
        cursor: 0,
    }
}

/// 开着的会话正在用 `mine`，名字的编辑窗开着、改成了「新名字」。
fn renaming(in_use: &str) -> Settings {
    let mut page = Settings::default();
    page.watch.persona = Some(in_use.into());
    let mut edit = LineEdit::open("persona.name", "名字", false, &Value::Null);
    edit.editor.insert("新名字");
    page.popup = Some(Popup::Line(edit.to(Target::Persona(Box::new(view())))));
    page
}

fn asked(page: &Settings) -> bool {
    matches!(&page.popup, Some(Popup::Confirm(c)) if c.ask == Delete::Held)
}

#[test]
fn saving_the_persona_in_use_asks_first_and_yes_saves() {
    let texts = texts();
    let mut page = renaming("mine");
    page.key(key(KeyCode::Enter), &texts);
    assert!(page.take_asks().is_empty(), "先问，不存");
    let Some(Popup::Confirm(confirm)) = &page.popup else {
        panic!("该问一句");
    };
    assert_eq!(confirm.lines[0].0, texts.in_use.persona, "项目主人的原话");
    page.key(key(KeyCode::Enter), &texts);
    let asks = page.take_asks();
    assert_eq!(asks.len(), 1);
    assert_eq!(asks[0].1, "persona.set");
    assert!(
        matches!(page.popup, Some(Popup::Persona(_))),
        "存完回到人格的窗"
    );
}

#[test]
fn no_keeps_the_typed_text_and_sends_nothing() {
    let texts = texts();
    for answer in [KeyCode::Esc, KeyCode::Down] {
        let mut page = renaming("mine");
        page.key(key(KeyCode::Enter), &texts);
        page.key(key(answer), &texts);
        if answer == KeyCode::Down {
            page.key(key(KeyCode::Enter), &texts);
        }
        assert!(page.take_asks().is_empty(), "答「否」不存");
        let Some(Popup::Line(edit)) = &page.popup else {
            panic!("回到编辑窗");
        };
        assert!(edit.editor.text().contains("新名字"), "填的不丢");
    }
}

#[test]
fn a_persona_not_in_use_or_before_a_session_saves_without_asking() {
    let texts = texts();
    for page in [renaming("other"), {
        let mut page = renaming("mine");
        page.watch.persona = None;
        page
    }] {
        let mut page = page;
        page.key(key(KeyCode::Enter), &texts);
        assert!(!asked(&page));
        assert_eq!(page.take_asks().len(), 1);
    }
}

#[test]
fn no_on_an_edited_prompt_hands_the_edited_text_back_to_the_editor() {
    let texts = texts();
    let mut page = Settings::default();
    page.watch.persona = Some("mine".into());
    page.prompt = Some(PromptEdit {
        persona: "mine".into(),
        prompt: "persona".into(),
        original: "你是她。".into(),
        version: None,
        view: view(),
    });
    page.prompt_edited(Some("你是另一个她。".into()));
    page.raise_held(&texts);
    assert!(asked(&page));
    page.key(key(KeyCode::Esc), &texts);
    assert!(page.take_asks().is_empty());
    assert_eq!(page.take_prompt().as_deref(), Some("你是另一个她。"));
}

#[test]
fn from_the_box_it_never_asks_and_remembers_what_was_saved() {
    let texts = texts();
    let mut page = renaming("mine");
    page.watch.lone = true;
    page.key(key(KeyCode::Enter), &texts);
    assert!(!asked(&page), "从框进来的存的时候不问");
    let (tag, _, _) = page.take_asks().remove(0);
    page.answer(tag, Ok(json!({"persona": {"id": "mine"}})), &texts);
    assert_eq!(page.watch.saved, [(Item::Persona, "mine".to_string())]);
}

#[test]
fn looks_only_and_refused_saves_are_not_remembered() {
    let texts = texts();
    let mut page = Settings::default();
    page.watch.lone = true;
    page.set_persona("mine", json!({"avatar": {"unset": true}}), false);
    let (tag, _, _) = page.take_asks().remove(0);
    page.answer(tag, Ok(json!({"persona": {"id": "mine"}})), &texts);
    page.take_asks();
    page.set_persona("mine", json!({"changes": []}), false);
    let (tag, method, _) = page.take_asks().remove(0);
    assert_eq!(method, "persona.set");
    let refused = crate::core::Refusal {
        message: "no".into(),
        reason: None,
        data: Value::Null,
    };
    page.answer(tag, Err(refused), &texts);
    assert!(page.watch.saved.is_empty());
}

#[test]
fn the_box_page_goes_back_once_the_detail_closes() {
    let mut page = Settings::one(true, Item::Preset, "full".into());
    assert!(!page.lone_done(), "详情窗开着");
    assert!(page.take_asks().iter().any(|a| a.1 == "preset.get"));
    page.popup = None;
    assert!(page.lone_done());
}
