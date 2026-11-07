//! 侧边栏「最近会话」当前这一行照谁（排期第 64 条，2026-11-07 真机验收看到）：切进子会话时照主会话，子会话不列；原来子会话
//! 不在列表里，被当成「还没进列表的（刚开的）」排到第一行、标成当前。

use super::app;

#[test]
fn the_current_row_is_the_main_session_even_while_a_child_is_viewed() {
    let (mut app, _sent) = app();
    app.talk.transcript.session = Some("main".into());
    app.talk.transcript.title = Some("主会话的标题".into());
    assert_eq!(
        app.talk.listed_current(),
        Some(("main", Some("主会话的标题")))
    );
    // 切进子会话：主会话停放着（`home`），正在看的是子会话。
    app.talk.home = Some("main".into());
    app.talk.transcript.session = Some("child".into());
    app.talk.transcript.title = Some("子会话".into());
    assert_eq!(app.talk.listed_current(), Some(("main", None)));
}
