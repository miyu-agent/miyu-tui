//! 「AI 怎么认识你」那一步（「第一次打开的引导」第 23b 条，2026-10-13 项目主人定，核心 P-9）。

use std::time::Instant;

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::{asks, find, fresh, key, texts, typed};
use crate::oobe::profile::Slot;
use crate::oobe::{Move, Oobe, Step, Texts};

/// 走到这一步，读到的三项是 `items`；配置清单里回复语言的建议值是简体中文、English。
fn at_profile(t0: Instant, texts: &Texts, items: serde_json::Value) -> Oobe {
    let mut oobe = fresh(t0);
    oobe.step = Step::Theme;
    oobe.moved(Move::Next, t0);
    assert_eq!(oobe.step, Step::Profile, "主题后面是 AI 怎么认识你");
    let sent = asks(&mut oobe);
    let (tag, params) = find(&sent, "config.get");
    assert_eq!(
        params,
        json!({"keys": ["profile.preferred_reply_language", "profile.name", "profile.about"]})
    );
    oobe.answer(tag, Ok(json!({"items": items})), t0, texts);
    let (tag, _) = find(&sent, "config.schema");
    let schema = json!({"items": [{"key": "profile.preferred_reply_language", "control": "combo",
        "suggestions": ["简体中文", "English"]}]});
    oobe.answer(tag, Ok(schema), t0, texts);
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
fn what_is_typed_is_written_to_the_settings_and_a_cleared_one_is_unset() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_profile(
        t0,
        &texts,
        json!({"profile.about": {"value": "写 Rust 的"}}),
    );
    assert_eq!(oobe.profile.about.text(), "写 Rust 的", "读到的填好");
    // 称呼：回车才编辑。
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
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
    assert_eq!(oobe.profile.focus, Slot::Next, "更多描述是最后一格");
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, params) = find(&asks(&mut oobe), "config.set");
    assert_eq!(
        params,
        json!({"changes": [
            {"key": "profile.name", "value": "小林"},
            {"key": "profile.about", "unset": true}
        ]})
    );
    assert_eq!(oobe.step, Step::Profile, "写成了才往下走");
    oobe.answer(tag, Ok(json!({})), t0, &texts);
    assert_eq!(oobe.step, Step::Preset);
}

#[test]
fn the_reply_language_is_picked_from_the_suggestions_or_typed_after_other() {
    // 核心 P-9 补（2026-10-16）：回复语言是下拉加「其他」手写，第一行「不设」（项目主人照推荐定）。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_profile(t0, &texts, json!({}));
    oobe.profile.focus = Slot::Language;
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let choose = oobe.profile.picking.as_ref().expect("开选择窗");
    let labels: Vec<&str> = choose.choices.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["不设", "简体中文", "English", "其他…"]);
    assert_eq!(choose.sel, 0, "没设的停在「不设」");
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(oobe.profile.picking.is_none(), "选了关窗");
    assert_eq!(oobe.profile.language.text(), "English");
    // 「其他…」：在这一格里手写，原来选的建议值不留。
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(oobe.profile.language.editing(), "「其他…」在格子里写");
    assert!(
        oobe.editing(),
        "手写回复语言算在打字，输入法照编辑里的开（2026-10-26 项目主人）"
    );
    assert_eq!(oobe.profile.language.text(), "");
    typed(&mut oobe, "粤语", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.profile.focus = Slot::Next;
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (_, params) = find(&asks(&mut oobe), "config.set");
    assert_eq!(
        params,
        json!({"changes": [
            {"key": "profile.preferred_reply_language", "value": "粤语"}
        ]})
    );
}

#[test]
fn the_reply_language_comes_first() {
    // 2026-10-16 项目主人定：「AI 用什么语言回复」排在「AI 怎么称呼你」上面，进这一步光标就在它上。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_profile(t0, &texts, json!({}));
    assert_eq!(oobe.profile.focus, Slot::Language);
    assert_eq!(texts.profile.fields[0], "AI 用什么语言回复");
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(
        oobe.profile.picking.is_some(),
        "第一格回车开回复语言的选择窗"
    );
    oobe.profile.picking = None;
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    assert_eq!(oobe.profile.focus, Slot::Name, "往下是称呼");
}
