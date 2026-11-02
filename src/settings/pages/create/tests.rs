//! 新建预设、人格的两格窗（「配置页」第 36、39 条子规则，2026-11-02 项目主人）：上面 id、下面名字；id 可空，写了的先查写法、
//! 查重；名字不能空。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use super::{Kind, valid_id};
use crate::config::Config;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts};

fn texts() -> Texts {
    Config::builtin().unwrap().text.settings
}

/// 配置页开好、人格列表里有 miyu。
fn opened() -> Settings {
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "persona.list" => Ok(json!({"personas": [{"persona": "miyu", "name": "美羽"}]})),
            _ => Ok(json!({})),
        };
        page.answer(tag, got, &texts());
    }
    page
}

fn typed(page: &mut Settings, text: &str) {
    for c in text.chars() {
        page.key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &texts(),
        );
    }
}

fn press(page: &mut Settings, code: KeyCode) {
    page.key(KeyEvent::new(code, KeyModifiers::NONE), &texts());
}

fn error(page: &Settings) -> Option<(String, usize)> {
    match &page.popup {
        Some(Popup::Create(edit)) => edit.error.clone().map(|e| (e, edit.focus)),
        _ => None,
    }
}

/// 交出去的 `persona.set` 的参数。
fn sent(page: &mut Settings) -> Option<Value> {
    page.take_asks()
        .into_iter()
        .find(|(_, method, _)| *method == "persona.set")
        .map(|(_, _, params)| params)
}

#[test]
fn the_id_comes_first_and_is_checked_before_the_name_is_sent() {
    let words = texts().more.create;
    let mut page = opened();
    page.open_create(Kind::Persona);
    // 不合写法：停在 id 那一格说为什么。
    typed(&mut page, "Writer");
    press(&mut page, KeyCode::Enter);
    typed(&mut page, "写手");
    press(&mut page, KeyCode::Enter);
    assert_eq!(error(&page), Some((words.id_bad.clone(), 0)));
    assert_eq!(sent(&mut page), None, "不交");
    // 和已有的重了。
    for _ in 0.."Writer".len() {
        press(&mut page, KeyCode::Backspace);
    }
    typed(&mut page, "miyu");
    press(&mut page, KeyCode::Enter);
    press(&mut page, KeyCode::Enter);
    assert_eq!(error(&page), Some((words.id_taken.clone(), 0)));
    // 换一个：交 `persona.set`，带 id 和名字，窗关上。
    for _ in 0.."miyu".len() {
        press(&mut page, KeyCode::Backspace);
    }
    typed(&mut page, "writer");
    press(&mut page, KeyCode::Enter);
    press(&mut page, KeyCode::Enter);
    let params = sent(&mut page).expect("交了");
    assert_eq!(params["persona"], "writer");
    assert_eq!(params["changes"][0]["value"], "写手");
    assert!(!matches!(page.popup, Some(Popup::Create(_))), "窗关上");
}

#[test]
fn an_empty_id_lets_the_core_pick_one_but_the_name_is_required() {
    let words = texts().more.create;
    let mut page = opened();
    page.open_create(Kind::Persona);
    press(&mut page, KeyCode::Enter);
    press(&mut page, KeyCode::Enter);
    assert_eq!(
        error(&page),
        Some((words.name_missing.clone(), 1)),
        "名字空着"
    );
    typed(&mut page, "写手");
    press(&mut page, KeyCode::Enter);
    let params = sent(&mut page).expect("交了");
    assert!(params.get("persona").is_none(), "不写 id，核心起：{params}");
}

#[test]
fn ids_follow_the_core_rules() {
    for good in ["writer", "w2", "a-b_c"] {
        assert!(valid_id(good), "{good}");
    }
    for bad in ["", "Writer", "2w", "-w", "写手", "a/b", &"a".repeat(65)] {
        assert!(!valid_id(bad), "{bad}");
    }
}

/// 列表还没读到：写了 id 的查不了重没重，不交（重了的会改掉已有的那一个）；id 空着的照交。
#[test]
fn an_id_is_not_sent_before_the_list_it_is_checked_against_has_come() {
    let words = texts().more.create;
    let mut page = Settings::open(false, true);
    drop(page.take_asks());
    page.open_create(Kind::Persona);
    typed(&mut page, "writer");
    press(&mut page, KeyCode::Enter);
    typed(&mut page, "写手");
    press(&mut page, KeyCode::Enter);
    assert_eq!(error(&page), Some((words.id_unchecked.clone(), 0)));
    assert_eq!(sent(&mut page), None, "不交");
    for _ in 0.."writer".len() {
        press(&mut page, KeyCode::Backspace);
    }
    press(&mut page, KeyCode::Enter);
    press(&mut page, KeyCode::Enter);
    assert!(sent(&mut page).is_some(), "id 空着的照交");
}

/// 从人格框按 `a` 进来（「新会话：人格、工作区」第 2c 条）：交出去还没回来时不算完（不回框），回来了开它的详情窗、记下
/// 新的编号，详情窗关了才回框。
#[test]
fn creating_from_the_pick_box_waits_for_the_new_one_and_remembers_it() {
    use crate::settings::in_use::Item;
    let mut page = Settings::create_one(true, Item::Persona);
    drop(page.take_asks());
    press(&mut page, KeyCode::Enter);
    typed(&mut page, "写手");
    press(&mut page, KeyCode::Enter);
    let (tag, method, _) = page
        .take_asks()
        .into_iter()
        .find(|(_, method, _)| *method == "persona.set")
        .expect("交了");
    assert_eq!(method, "persona.set");
    assert!(page.popup.is_none(), "两格窗关了");
    assert!(!page.lone_done(), "新建的还没回来：不回框");
    let got = json!({"persona": "persona-1", "name": "写手"});
    page.answer(tag, Ok(got), &texts());
    assert!(
        matches!(page.popup, Some(Popup::Persona(_))),
        "开它的详情窗"
    );
    assert_eq!(
        page.watch.created,
        Some((Item::Persona, "persona-1".to_string()))
    );
    assert!(!page.lone_done());
    press(&mut page, KeyCode::Esc);
    assert!(page.lone_done(), "详情窗关了：回框");
}
