//! 引导里断线、重连（review B6，2026-10-11）：动作照被拒办，读的复位、重连时再要。原来只是忘掉等着的：在试的不认任何键，
//! 人格、预设两步一直忙着，最后一屏回车以后关不掉，只能 Ctrl+C 退出。

use std::time::{Duration, Instant};

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::{asks, find, fresh, key, texts, to_model};
use crate::oobe::model::Phase;
use crate::oobe::{Move, Step};

const LOST: &str = "核心断开连接";

#[test]
fn a_test_cut_off_goes_back_to_the_form_and_says_why() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    to_model(&mut oobe, t0, &texts);
    for (tag, method, _) in asks(&mut oobe) {
        let reply = match method {
            "model.list" => json!({"uses": {"chat": null}, "providers": []}),
            "provider.detect" => json!({"keys": [
                {"env": "DEEPSEEK_API_KEY", "provider": "deepseek", "supported": true}]}),
            "provider.catalog" => json!({"providers": [
                {"id": "deepseek", "name": "DeepSeek", "supported": true}]}),
            _ => json!({"items": {}}),
        };
        oobe.answer(tag, Ok(reply), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(matches!(oobe.model.phase, Phase::Testing(_)));
    oobe.offline(LOST, t0, &texts);
    assert!(
        !matches!(oobe.model.phase, Phase::Testing(_)),
        "{:?}",
        oobe.model.phase
    );
    assert!(
        oobe.model
            .error
            .as_deref()
            .is_some_and(|e| e.contains(LOST))
    );
}

#[test]
fn a_persona_read_cut_off_is_read_again_after_reconnecting() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    oobe.step = Step::Currency;
    oobe.moved(Move::Next, t0);
    assert_eq!(oobe.step, Step::Persona);
    find(&asks(&mut oobe), "config.get");
    assert!(oobe.persona.busy);
    oobe.offline(LOST, t0, &texts);
    assert!(!oobe.persona.busy, "不再一直忙着");
    oobe.online();
    let (_, params) = find(&asks(&mut oobe), "config.get");
    assert_eq!(params, json!({"keys": ["persona.default"]}));
}

#[test]
fn the_last_enter_cut_off_still_closes_and_says_it_was_not_recorded() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    oobe.step = Step::Preset;
    oobe.moved(Move::Next, t0);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    find(&asks(&mut oobe), "config.set");
    oobe.offline(LOST, t0, &texts);
    let leave = Duration::from_millis(oobe.look.done.leave_ms);
    assert_eq!(
        oobe.closing(t0 + leave, leave),
        Some(Some(LOST.to_string()))
    );
}
