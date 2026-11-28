//! 切会话（`switch.rs`）：点了还没换上来又点一下。

use super::app;
use crate::core::Command;

#[test]
fn switching_to_a_session_still_opening_waits_for_it() {
    // 2026-10-27 项目主人报：侧边栏切会话有时一闪空会话的画面。第二下原来把停放着、还在补发的空正文提前换了上来。
    let (mut app, mut sent) = app();
    app.talk.transcript.session = Some("s-two".into());
    app.switch_session("s-one".into());
    app.switch_session("s-one".into());
    assert_eq!(
        app.talk.transcript.session.as_deref(),
        Some("s-two"),
        "补完以前照旧看着原来的"
    );
    assert!(app.talk.opening("s-one"));
    let mut opens = 0;
    while let Ok(command) = sent.try_recv() {
        if matches!(command, Command::Open { .. }) {
            opens += 1;
        }
    }
    assert_eq!(opens, 1, "只开一次");
}

#[test]
fn a_new_session_before_the_switch_lands_can_talk() {
    // 2026-11-28 审计第 2 条（a）：切到 s-one、还没读完就 `/new`，核心那头不再对着 s-one，补完的那一下不会来；原来
    // `opening` 一直留着，界面一直「正在切换会话」，发不出话。
    let (mut app, mut sent) = app();
    app.talk.transcript.session = Some("s-two".into());
    app.switch_session("s-one".into());
    app.new_session();
    assert!(!app.talk.opening("s-one"), "不再切过去");
    assert!(
        !app.talk.parked.contains_key("s-one"),
        "停放着等补发的空正文不要了"
    );
    while sent.try_recv().is_ok() {}
    app.submit(crate::input::Draft::plain("你好"));
    assert!(
        std::iter::from_fn(|| sent.try_recv().ok()).any(|c| matches!(c, Command::Send { .. })),
        "照常发出去"
    );
}

#[test]
fn a_new_session_before_the_switch_lands_drops_the_session_it_was_opening() {
    // 同上：原来看着的那个在跑（`keep`），`/new` 照它交 `keep`，核心把还在读的 s-one 当成在跑的留着订阅。
    let (mut app, mut sent) = app();
    app.talk.transcript.session = Some("s-two".into());
    app.talk.transcript.running = Some(std::time::Instant::now());
    app.switch_session("s-one".into());
    while sent.try_recv().is_ok() {}
    app.new_session();
    let news: Vec<bool> = std::iter::from_fn(|| sent.try_recv().ok())
        .filter_map(|c| match c {
            Command::New { keep } => Some(keep),
            _ => None,
        })
        .collect();
    assert_eq!(news, [false], "核心那头不再订阅 s-one");
    assert!(
        app.talk.parked.contains_key("s-two"),
        "原来在跑的照切走时说的停放着"
    );
}
