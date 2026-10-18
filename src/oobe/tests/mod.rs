//! 引导走到哪、按键、回应（「第一次打开的引导」）：拿假钟一步步走，查发给核心的请求和交给 App 办的。

mod figure;
mod link;
mod persona;
mod preset;
mod profile;
mod skip;
mod theme;
mod theme_seed;

use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use super::pick::Pick;
use super::{Effect, Oobe, Step, Texts};
use crate::config::Config;

pub(super) fn texts() -> Texts {
    Config::builtin().unwrap().text.oobe
}

pub(super) fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

pub(super) fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

pub(super) fn typed(oobe: &mut Oobe, text: &str, now: Instant, texts: &Texts) {
    for c in text.chars() {
        oobe.key(key(KeyCode::Char(c)), now, texts);
    }
}

/// 新起一个引导，停在欢迎页、开场播完。
pub(super) fn fresh(now: Instant) -> Oobe {
    let look = Config::builtin().unwrap().oobe;
    let rows = vec![
        "跟随系统（中文）".into(),
        "中文".into(),
        "English".into(),
        "日本語".into(),
    ];
    let mut oobe = Oobe::new(now, look, Pick::new(rows, 0), "nerd", true);
    oobe.intro.skip();
    // 没装语义模型的包：那一步跨过去（装了的走法见 `the_semantic_step_shows_only_with_the_package`）。
    oobe.semantic.installed = Some(false);
    oobe
}

/// 发出去的请求：(方法, 参数)，连同编号。
pub(super) fn asks(oobe: &mut Oobe) -> Vec<(u64, &'static str, Value)> {
    oobe.take_asks()
        .into_iter()
        .map(|a| (a.tag, a.method, a.params))
        .collect()
}

/// 照方法找一条发出去的，交回编号和参数。
pub(super) fn find(list: &[(u64, &'static str, Value)], method: &str) -> (u64, Value) {
    let (tag, _, params) = list
        .iter()
        .find(|(_, m, _)| *m == method)
        .unwrap_or_else(|| panic!("没发 {method}：{list:?}"));
    (*tag, params.clone())
}

/// 走到接模型那一步。
pub(super) fn to_model(oobe: &mut Oobe, now: Instant, texts: &Texts) {
    oobe.key(key(KeyCode::Enter), now, texts);
    oobe.key(key(KeyCode::Enter), now, texts);
    oobe.key(key(KeyCode::Enter), now, texts);
    assert_eq!(oobe.step, Step::Model);
}

#[test]
fn a_key_during_the_intro_only_skips_it_then_enter_walks_through_language_and_icons() {
    let texts = texts();
    let t0 = Instant::now();
    let look = Config::builtin().unwrap().oobe;
    let rows = vec!["跟随系统（中文）".into(), "中文".into()];
    let mut oobe = Oobe::new(t0, look, Pick::new(rows, 0), "plain", true);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert_eq!(oobe.step, Step::Welcome, "开场时按回车只是跳过");
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert_eq!(oobe.step, Step::Language);
    assert!(oobe.sliding.is_some(), "滑过去");
    // 语言：往下一行回车，交给 App 换成中文。
    oobe.key(key(KeyCode::Down), t0, &texts);
    let effects = oobe.key(key(KeyCode::Enter), t0, &texts);
    assert_eq!(effects, [Effect::Language(1)]);
    assert_eq!(oobe.step, Step::Icons);
    assert_eq!(oobe.icons.selected, 1, "开的时候选着现在用的 plain");
    oobe.key(key(KeyCode::Up), t0, &texts);
    let effects = oobe.key(key(KeyCode::Enter), t0, &texts);
    assert_eq!(effects, [Effect::Icons("nerd".into())]);
    let sent = asks(&mut oobe);
    let (_, params) = find(&sent, "config.set");
    assert_eq!(
        params,
        json!({"layer": "personal", "changes": [{"key": "tui.icons", "value": "nerd"}]})
    );
    // 进了接模型：要三份回应和配置里有没有池。
    assert_eq!(oobe.step, Step::Model);
    for method in [
        "model.list",
        "provider.detect",
        "provider.catalog",
        "config.get",
    ] {
        find(&sent, method);
    }
    assert_eq!(find(&sent, "provider.catalog").1, json!({"featured": true}));
}

#[test]
fn esc_goes_back_a_step_and_the_welcome_page_does_not_replay() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(key(KeyCode::Esc), t0, &texts);
    assert_eq!(oobe.step, Step::Welcome);
    assert!(
        oobe.intro
            .finished(t0, &oobe.look.intro, texts.welcome.title.chars().count()),
        "回来不再播开场"
    );
    assert_eq!(oobe.step.bar(), None);
    assert_eq!(Step::Language.bar(), Some(0));
    assert_eq!(Step::Theme.bar(), Some(4));
    assert_eq!(Step::Preset.bar(), Some(6));
}

#[test]
fn the_last_screen_marks_welcomed_and_closes_after_the_stars_leave() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    oobe.step = Step::Preset;
    oobe.moved(super::Move::Next, t0);
    assert_eq!(oobe.step, Step::Done);
    // 播着时回车只是跳过。
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(oobe.leaving.is_none());
    assert!(oobe.finale(t0, &texts).ready);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(oobe.leaving.is_some());
    let sent = asks(&mut oobe);
    let (tag, params) = find(&sent, "config.set");
    assert_eq!(
        params["changes"],
        json!([{"key": "ui.welcomed", "value": true}])
    );
    let leave = Duration::from_millis(oobe.look.done.leave_ms);
    assert_eq!(oobe.closing(t0 + leave, leave), None, "写标记的还没回来");
    assert!(oobe.answer(tag, Ok(json!({})), t0, &texts));
    assert_eq!(oobe.closing(t0, leave), None, "散开还没播完");
    assert_eq!(oobe.closing(t0 + leave, leave), Some(None));
}

#[test]
fn offline_the_last_enter_still_leaves_and_says_it_was_not_recorded() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    oobe.step = Step::Preset;
    oobe.moved(super::Move::Next, t0);
    oobe.done_at = t0.checked_sub(Duration::from_secs(60));
    oobe.offline("核心断开连接", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let leave = Duration::from_millis(oobe.look.done.leave_ms);
    assert!(matches!(oobe.closing(t0 + leave, leave), Some(Some(_))));
}

#[test]
fn enter_after_the_intro_has_played_out_starts_right_away() {
    let texts = texts();
    let t0 = Instant::now();
    let look = Config::builtin().unwrap().oobe;
    let mut oobe = Oobe::new(t0, look, Pick::new(vec!["a".into()], 0), "nerd", true);
    let later = t0 + Duration::from_secs(30);
    assert!(
        oobe.intro
            .finished(later, &oobe.look.intro, texts.welcome.title.chars().count())
    );
    oobe.key(key(KeyCode::Enter), later, &texts);
    assert_eq!(oobe.step, Step::Language, "播完了第一下回车就开始");
}

#[test]
fn the_semantic_step_shows_only_with_the_package() {
    // 2026-10-18 项目主人：引导最后一步问开不开内置的语义模型，只在装了它的包时出现。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    oobe.semantic.installed = None;
    oobe.step = Step::Profile;
    oobe.moved(super::Move::Next, t0);
    assert_eq!(oobe.step, Step::Preset);
    let sent = asks(&mut oobe);
    let (schema, _) = find(&sent, "config.schema");
    let local = json!({"items": [{"key": "models.embedding", "options": [{"value": "local"}, {"value": "off"}]}]});
    oobe.answer(schema, Ok(local), t0, &texts);
    oobe.moved(super::Move::Next, t0);
    assert_eq!(oobe.step, Step::Semantic, "装了的出现在预设后面");
    let (read, _) = find(&asks(&mut oobe), "config.get");
    oobe.answer(read, Ok(json!({"items": {}})), t0, &texts);
    oobe.key(key(KeyCode::Down), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, params) = find(&asks(&mut oobe), "config.set");
    assert_eq!(
        params,
        json!({"layer": "personal", "changes": [{"key": "models.embedding", "value": "off"}]}),
        "「否」只用关键词"
    );
    oobe.answer(tag, Ok(json!({})), t0, &texts);
    assert_eq!(oobe.step, Step::Done);
}

#[test]
fn keeping_the_semantic_setting_as_it_is_writes_nothing() {
    // 同网页（2026-10-18 shorin-ee）：选的和现在的一样不写个人设置；没写过的算「是」。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    oobe.semantic.installed = Some(true);
    oobe.step = Step::Preset;
    oobe.moved(super::Move::Next, t0);
    assert_eq!(oobe.step, Step::Semantic);
    let (read, _) = find(&asks(&mut oobe), "config.get");
    oobe.answer(read, Ok(json!({"items": {}})), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(
        asks(&mut oobe).iter().all(|(_, m, _)| *m != "config.set"),
        "没改的不写"
    );
    assert_eq!(oobe.step, Step::Done);
}
