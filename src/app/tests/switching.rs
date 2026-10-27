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
