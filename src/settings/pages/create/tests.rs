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
