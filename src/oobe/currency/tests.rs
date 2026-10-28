use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::json;

use super::{CURRENCIES, CurrencyStep};
use crate::oobe::Move;
use crate::oobe::asker::Asker;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn it_stops_on_what_was_set_or_on_the_languages_default() {
    let mut step = CurrencyStep::default();
    let shipped =
        json!({"items": {"usage.currency": {"value": "USD", "origin": {"layer": "default"}}}});
    step.answer(&Ok(shipped), "CNY");
    assert_eq!(
        CURRENCIES[step.pick.selected], "CNY",
        "没人设过：照界面语言"
    );
    let set = json!({"items": {"usage.currency": {"value": "EUR", "origin": {"layer": "system"}}}});
    step.answer(&Ok(set), "CNY");
    assert_eq!(CURRENCIES[step.pick.selected], "EUR", "设过的停在那一种");
}

#[test]
fn enter_writes_the_setting_and_moves_on() {
    let mut step = CurrencyStep::default();
    let mut asker = Asker::default();
    step.key(key(KeyCode::Down), &mut asker);
    assert_eq!(step.key(key(KeyCode::Enter), &mut asker), Move::Next);
    let sent = asker.take();
    let (_, method, params) = sent
        .iter()
        .map(|a| (a.tag, a.method, a.params.clone()))
        .find(|(_, m, _)| *m == "config.set")
        .expect("写系统配置");
    assert_eq!(method, "config.set");
    assert_eq!(
        params,
        json!({"changes": [{"key": "usage.currency", "value": "USD"}]})
    );
    assert_eq!(step.key(key(KeyCode::Esc), &mut asker), Move::Back);
}

#[test]
fn other_takes_a_three_letter_code_and_refuses_anything_else() {
    // 2026-10-27 项目主人：「列更多常见的，然后加其他」。
    let mut step = CurrencyStep::default();
    let mut asker = Asker::default();
    for _ in 0..CURRENCIES.len() {
        step.key(key(KeyCode::Down), &mut asker);
    }
    assert_eq!(step.pick.selected, step.other_row(), "最后一行是「其他…」");
    assert_eq!(step.key(key(KeyCode::Enter), &mut asker), Move::Stay);
    assert!(step.other.editing(), "回车开始填");
    for c in "sgdx".chars() {
        step.key(key(KeyCode::Char(c)), &mut asker);
    }
    assert_eq!(step.key(key(KeyCode::Enter), &mut asker), Move::Stay);
    assert!(
        step.invalid && step.other.editing(),
        "四个字母：红字，接着改"
    );
    step.key(key(KeyCode::Backspace), &mut asker);
    assert_eq!(step.key(key(KeyCode::Enter), &mut asker), Move::Next);
    let sent = asker.take();
    let params = sent
        .iter()
        .find(|a| a.method == "config.set")
        .map(|a| a.params.clone())
        .expect("写系统配置");
    assert_eq!(params["changes"][0]["value"], "SGD", "转大写");
}

#[test]
fn a_code_not_in_the_list_opens_on_other() {
    let mut step = CurrencyStep::default();
    let set = json!({"items": {"usage.currency": {"value": "BRL", "origin": {"layer": "system"}}}});
    step.answer(&Ok(set), "CNY");
    assert_eq!(step.pick.selected, step.other_row());
    assert_eq!(step.other.text(), "BRL");
    assert_eq!(super::code(" brl "), Some("BRL".into()));
    assert_eq!(super::code("B1L"), None);
}
