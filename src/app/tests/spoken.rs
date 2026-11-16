//! 会话列表的先后（2026-11-16 项目主人）：只切进去不挪，人发了话才挪到顶上；循环、定时任务、人生自己醒这些没人说话的轮
//! 不算；没人说过话的照开的时刻。侧边栏「最近会话」照手上这一份会话列表的先后列（`ui/recent.rs`）。

use super::app;
use crate::app::App;
use crate::core::{SessionChange, SessionInfo};

/// 开在第 `minute` 分钟的会话的编号：UUIDv7，前 48 位是开的那一毫秒。
fn id(minute: i64, tag: char) -> String {
    let ms = at(minute).as_millisecond();
    let tail: String = std::iter::repeat_n(tag, 12).collect();
    format!("{:08x}-{:04x}-7000-8000-{tail}", ms >> 16, ms & 0xffff)
}

/// 2026-11-16 零点起第 `minute` 分钟。
fn at(minute: i64) -> jiff::Timestamp {
    let start: jiff::Timestamp = "2026-11-16T00:00:00Z".parse().unwrap();
    start + jiff::SignedDuration::from_mins(minute)
}

fn info(session: &str, spoken: Option<i64>, active: i64) -> SessionInfo {
    SessionInfo {
        session: session.into(),
        last_spoken: spoken.map(at),
        last_active: Some(at(active)),
        ..SessionInfo::default()
    }
}

fn order(app: &App) -> Vec<String> {
    let seen = app.talk.sessions_seen.as_deref().unwrap_or_default();
    seen.iter().map(|s| s.session.clone()).collect()
}

fn changed(app: &mut App, entry: SessionInfo) {
    app.session_changed(SessionChange::Entry(Box::new(entry)));
}

#[test]
fn switching_into_a_session_does_not_move_it() {
    let (mut app, _sent) = app();
    let (a, b, c) = (id(1, 'a'), id(2, 'b'), id(3, 'c'));
    // 核心照最近动静交回：c 刚被循环叫醒过，排在最前。
    app.sessions_listed(vec![
        info(&c, Some(10), 50),
        info(&a, Some(30), 30),
        info(&b, Some(20), 20),
    ]);
    let before = vec![a.clone(), b.clone(), c.clone()];
    assert_eq!(order(&app), before, "照人最后一次发话");
    app.talk.transcript.session = Some(a.clone());
    app.switch_session(c.clone());
    // 切过去：核心推一项 `viewed`，动静也可能跟着动。
    changed(
        &mut app,
        SessionInfo {
            viewed: true,
            ..info(&c, Some(10), 51)
        },
    );
    assert_eq!(order(&app), before, "切进去不挪");
}

#[test]
fn a_person_speaking_moves_the_session_to_the_top() {
    let (mut app, _sent) = app();
    let (a, b, c) = (id(1, 'a'), id(2, 'b'), id(3, 'c'));
    app.sessions_listed(vec![
        info(&a, Some(30), 30),
        info(&b, Some(20), 20),
        info(&c, Some(10), 10),
    ]);
    changed(&mut app, info(&c, Some(40), 40));
    assert_eq!(order(&app), [c, a, b]);
}

#[test]
fn a_round_nobody_spoke_in_does_not_move_it() {
    let (mut app, _sent) = app();
    let (a, b, c) = (id(1, 'a'), id(2, 'b'), id(3, 'c'));
    app.sessions_listed(vec![
        info(&a, Some(30), 30),
        info(&b, Some(20), 20),
        info(&c, Some(10), 10),
    ]);
    let before = vec![a.clone(), b.clone(), c.clone()];
    // 循环、定时任务、人生自己醒：只有最近动静变了。
    changed(&mut app, info(&c, Some(10), 60));
    assert_eq!(order(&app), before, "推来的一项不挪");
    // 重连、置顶以后核心整份交回，照最近动静排的：c 在最前。
    app.sessions_listed(vec![
        info(&c, Some(10), 60),
        info(&a, Some(30), 30),
        info(&b, Some(20), 20),
    ]);
    assert_eq!(order(&app), before, "整份重读也不挪");
}

#[test]
fn sessions_nobody_spoke_in_sort_by_when_they_were_opened() {
    let (mut app, _sent) = app();
    let spoke_late = id(1, 'a');
    let spoke_early = id(2, 'b');
    let opened_late = id(40, 'c');
    let opened_early = id(20, 'd');
    // 没人说过话的人生会话自己醒得最晚，也不照最近动静往上挪。
    app.sessions_listed(vec![
        info(&opened_early, None, 90),
        info(&spoke_early, Some(10), 10),
        info(&opened_late, None, 41),
        info(&spoke_late, Some(30), 30),
    ]);
    assert_eq!(
        order(&app),
        [opened_late, spoke_late, opened_early, spoke_early]
    );
}
