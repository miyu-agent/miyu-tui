//! 「AI 怎么认识你」那一步（「第一次打开的引导」第 23a 条，2026-10-13 项目主人定，核心 P-9）。

use std::time::Instant;

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::{asks, find, fresh, key, texts, typed};
use crate::oobe::profile::Slot;
use crate::oobe::{Move, Oobe, Step, Texts};

/// 走到这一步，读到的两项是 `items`。
fn at_profile(t0: Instant, texts: &Texts, items: serde_json::Value) -> Oobe {
    let mut oobe = fresh(t0);
    oobe.step = Step::Persona;
    oobe.moved(Move::Next, t0);
    assert_eq!(oobe.step, Step::Profile);
    let (tag, params) = find(&asks(&mut oobe), "config.get");
    assert_eq!(params, json!({"keys": ["profile.name", "profile.about"]}));
    oobe.answer(tag, Ok(json!({"items": items})), t0, texts);
    oobe
}

#[test]
fn both_can_be_left_empty_and_nothing_is_written() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_profile(t0, &texts, json!({}));
    oobe.profile.focus = Slot::Next;
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(
        asks(&mut oobe).iter().all(|(_, m, _)| *m != "config.set"),
        "都空着：不写配置"
    );
    assert_eq!(oobe.step, Step::Preset);
}

#[test]
fn what_is_typed_is_written_to_personal_settings_and_a_cleared_one_is_unset() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_profile(
        t0,
        &texts,
        json!({"profile.about": {"value": "写 Rust 的"}}),
    );
    assert_eq!(oobe.profile.about.text(), "写 Rust 的", "读到的填好");
    // 称呼：回车才编辑。
    oobe.key(key(KeyCode::Enter), t0, &texts);
    typed(&mut oobe, "小林", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    // 更多描述：在大编辑浮窗里清空，Esc 收起。
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(oobe.profile.writing.is_some(), "更多描述在大编辑浮窗里写");
    for _ in 0.."写 Rust 的".chars().count() {
        oobe.key(key(KeyCode::Backspace), t0, &texts);
    }
    oobe.key(key(KeyCode::Esc), t0, &texts);
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, params) = find(&asks(&mut oobe), "config.set");
    assert_eq!(
        params,
        json!({"layer": "personal", "changes": [
            {"key": "profile.name", "value": "小林"},
            {"key": "profile.about", "unset": true}
        ]})
    );
    assert_eq!(oobe.step, Step::Profile, "写成了才往下走");
    oobe.answer(tag, Ok(json!({})), t0, &texts);
    assert_eq!(oobe.step, Step::Preset);
}
