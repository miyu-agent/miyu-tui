//! 按键模式是 Vim 时「更多供应商…」里的搜索：删光了再退格不退出（发版前 check 50）。

use std::time::Instant;

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::at_model;
use crate::oobe::tests::{asks, find, key, texts, typed};

#[test]
fn with_vim_keys_backspace_on_an_empty_search_stays_in_the_search() {
    // 发版前 check 50（2026-11-12 项目主人）：按 / 进了搜索，退格删光了再按，不该把搜索退掉，很容易误触；只有 Esc 退出。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_model(t0, &texts, None);
    oobe.model.vim = true;
    for _ in 0..2 {
        oobe.key(key(KeyCode::Down), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, _) = find(&asks(&mut oobe), "provider.catalog");
    let all = json!({"providers": [{"id": "zai", "name": "Z.AI", "supported": true}]});
    oobe.answer(tag, Ok(all), t0, &texts);
    oobe.key(key(KeyCode::Char('/')), t0, &texts);
    typed(&mut oobe, "z", t0, &texts);
    for _ in 0..3 {
        oobe.key(key(KeyCode::Backspace), t0, &texts);
    }
    let more = oobe.model.more.as_ref().expect("窗还开着");
    assert!(more.filter.editing(), "删光了再退格还在搜索里");
    oobe.key(key(KeyCode::Esc), t0, &texts);
    let more = oobe.model.more.as_ref().expect("Esc 退出搜索，窗还开着");
    assert!(!more.filter.editing());
}
