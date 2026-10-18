//! 什么都没填的「下一步」写成「跳过」（2026-10-19 项目主人，同网页）：人格那一步跳过不建、主题那一步跟着跳过、好了那一屏
//! 不写人格；「AI 怎么认识你」跳过什么都不写。

use std::time::Instant;

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::{asks, find, fresh, key, texts};
use crate::oobe::{Move, Step};

#[test]
fn an_empty_persona_is_skipped_and_the_theme_step_with_it() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    oobe.step = Step::Persona;
    assert!(oobe.persona.blank());
    oobe.persona.focus = crate::oobe::persona::Slot::Next;
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert_eq!(oobe.step, Step::Profile, "主题跟着跳过");
    assert!(
        asks(&mut oobe).iter().all(|(_, m, _)| *m != "persona.set"),
        "不建人格"
    );
    oobe.moved(Move::Back, t0);
    assert_eq!(oobe.step, Step::Persona, "往回也跨过主题");
    oobe.persona.name = crate::oobe::field::Field::line().with("小雪");
    assert!(!oobe.persona.blank(), "填了的照旧是「下一步」");
}

#[test]
fn an_empty_profile_is_skipped_without_writing_even_what_was_there() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    oobe.persona.chosen_id = Some("p1".into());
    oobe.step = Step::Theme;
    oobe.moved(Move::Next, t0);
    let sent = asks(&mut oobe);
    let (tag, _) = find(&sent, "config.get");
    let items = json!({"profile.name": {"value": "小明"}});
    oobe.answer(tag, Ok(json!({"items": items})), t0, &texts);
    assert!(!oobe.profile.blank(), "读到了的不是空的");
    oobe.profile.name = crate::oobe::field::Field::line();
    assert!(oobe.profile.blank(), "清空了是「跳过」");
    oobe.profile.focus = crate::oobe::profile::Slot::Next;
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(
        asks(&mut oobe).iter().all(|(_, m, _)| *m != "config.set"),
        "跳过什么都不写"
    );
    assert_eq!(oobe.step, Step::Preset);
}
