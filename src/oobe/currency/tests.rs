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
    let set =
        json!({"items": {"usage.currency": {"value": "EUR", "origin": {"layer": "personal"}}}});
    step.answer(&Ok(set), "CNY");
    assert_eq!(CURRENCIES[step.pick.selected], "EUR", "设过的停在那一种");
}

#[test]
fn enter_writes_the_personal_setting_and_moves_on() {
    let mut step = CurrencyStep::default();
    let mut asker = Asker::default();
    step.key(key(KeyCode::Down), &mut asker);
    assert_eq!(step.key(key(KeyCode::Enter), &mut asker), Move::Next);
    let sent = asker.take();
    let (_, method, params) = sent
        .iter()
        .map(|a| (a.tag, a.method, a.params.clone()))
        .find(|(_, m, _)| *m == "config.set")
        .expect("写个人设置");
    assert_eq!(method, "config.set");
    assert_eq!(
        params,
        json!({"layer": "personal", "changes": [{"key": "usage.currency", "value": "USD"}]})
    );
    assert_eq!(step.key(key(KeyCode::Esc), &mut asker), Move::Back);
}
