//! 你说的话的竖线照发出去那一刻的权限级别上色，之后换了级别也不变（蓝图 `tui.md`「正文」第 2 条）：新开的会话、
//! `--resume` 进的会话、从列表切过去的会话、往上翻读进来的更早的一页、一页里两句话之间换过级别的，都照核心的事件走一遍。
//! 2026-11-16 项目主人报：按 Shift+Tab 换了级别，已经发出去的话的竖线跟着变了色。

use serde_json::{Value, json};

use super::app;
use crate::app::App;
use crate::core::{Level, Push, Snapshot, Update, read_replayed};
use crate::transcript::Kind;

const AT: &str = "2026-11-16T08:00:00Z";

fn created(read_only: bool) -> Value {
    json!({"seq": 1, "at": AT, "kind": "session.created", "by": {"kind": "person"},
           "body": {"venue": "local", "permission": {"level": "full", "read_only": read_only},
                    "persona": "miyu", "preset": "work", "model": "a/b", "cwd": "~/a"}})
}

fn policy(seq: u64, read_only: bool) -> Value {
    json!({"seq": seq, "at": AT, "kind": "session.policy_changed", "by": {"kind": "person"},
           "body": {"permission": {"level": "full", "read_only": read_only}}})
}

/// 第 `seq` 条你说 `said`，接着一轮：开轮、收尾。
fn turn(seq: u64, said: &str) -> Vec<Value> {
    vec![
        json!({"seq": seq, "at": AT, "kind": "message.user", "by": {"kind": "person"},
               "body": {"blocks": [{"type": "text", "text": said}]}}),
        json!({"seq": seq + 1, "at": AT, "kind": "turn.started", "turn": seq + 1,
               "body": {"trigger": seq}}),
        json!({"seq": seq + 2, "at": AT, "kind": "turn.ended", "turn": seq + 1,
               "body": {"reason": "completed"}}),
    ]
}

fn page(events: Vec<Vec<Value>>) -> Vec<Push> {
    read_replayed(&events.concat())
}

fn to(app: &mut App, session: &str, update: Update) {
    app.core(Update::Elsewhere {
        session: session.into(),
        update: Box::new(update),
    });
}

fn snapshot(level: Level) -> Update {
    Update::Snapshot(Snapshot {
        level: Some(level),
        ..Snapshot::default()
    })
}

/// 看着的会话里你说的话、收尾行各记着什么级别。
fn levels(app: &App) -> Vec<(String, Option<Level>)> {
    app.talk
        .transcript
        .entries
        .iter()
        .filter(|e| matches!(e.kind, Kind::User | Kind::Done))
        .map(|e| (e.text.clone(), e.level))
        .collect()
}

fn said(app: &App) -> Vec<(String, Option<Level>)> {
    app.talk
        .transcript
        .entries
        .iter()
        .filter(|e| e.kind == Kind::User)
        .map(|e| (e.text.clone(), e.level))
        .collect()
}

/// 进已有的会话：最新一页、更早的还有没有、订阅回应，照 `--resume` 的先后。
fn resume(app: &mut App, pushes: Vec<Push>, more: bool, now: Level) {
    app.core(Update::Ready("s1".into()));
    for push in pushes {
        to(app, "s1", Update::Push(push));
    }
    to(app, "s1", Update::Paged(more));
    to(app, "s1", snapshot(now));
}

fn live_switch(app: &mut App, read_only: bool) {
    let push = crate::core::read_live(&[policy(900, read_only)]);
    for push in push {
        app.core(Update::Push(push));
    }
}

#[test]
fn a_resumed_session_keeps_the_bars_when_the_level_changes() {
    let (mut app, _sent) = app();
    resume(
        &mut app,
        page(vec![turn(10, "一"), turn(20, "二")]),
        true,
        Level::Full,
    );
    let full = Some(Level::Full);
    assert!(
        levels(&app).iter().all(|(_, level)| *level == full),
        "这一页没换过级别：照订阅回应的现在的级别：{:?}",
        levels(&app)
    );
    live_switch(&mut app, true);
    assert!(levels(&app).iter().all(|(_, level)| *level == full));
}

#[test]
fn a_session_opened_from_the_list_keeps_the_bars_when_the_level_changes() {
    let (mut app, _sent) = app();
    app.core(Update::Ready("s0".into()));
    app.switch_session("s1".into());
    for push in page(vec![turn(10, "一")]) {
        to(&mut app, "s1", Update::Push(push));
    }
    to(&mut app, "s1", Update::Paged(true));
    to(&mut app, "s1", snapshot(Level::ReadOnly));
    to(&mut app, "s1", Update::Push(Push::Clock(None)));
    assert_eq!(app.talk.transcript.session.as_deref(), Some("s1"));
    assert_eq!(said(&app), [("一".to_string(), Some(Level::ReadOnly))]);
    live_switch(&mut app, false);
    assert_eq!(said(&app), [("一".to_string(), Some(Level::ReadOnly))]);
}

#[test]
fn a_level_change_between_two_messages_in_one_page_splits_their_colors() {
    // 这一页从会话开头起：开头的级别在 `session.created` 里。
    let (mut app, _sent) = app();
    let events = vec![
        vec![created(false)],
        turn(10, "一"),
        vec![policy(15, true)],
        turn(20, "二"),
    ];
    resume(&mut app, page(events), false, Level::ReadOnly);
    let full = Some(Level::Full);
    let read_only = Some(Level::ReadOnly);
    assert_eq!(
        said(&app),
        [("一".to_string(), full), ("二".to_string(), read_only)]
    );
    live_switch(&mut app, false);
    assert_eq!(
        said(&app),
        [("一".to_string(), full), ("二".to_string(), read_only)]
    );
}

#[test]
fn messages_before_the_first_change_in_the_page_wait_for_the_older_page_then_stay() {
    // 这一页开头的级别不知道（换级别的那一条在更早的页里）：先不照现在的画，更早的一页读进来照它的末尾补上。
    let (mut app, _sent) = app();
    let events = vec![turn(30, "三"), vec![policy(35, true)], turn(40, "四")];
    resume(&mut app, page(events), true, Level::ReadOnly);
    let before = said(&app);
    live_switch(&mut app, false);
    assert_eq!(said(&app), before, "换了级别，已经发出去的不变");
    assert_eq!(before[1], ("四".to_string(), Some(Level::ReadOnly)));
    let older = page(vec![
        vec![created(true)],
        turn(10, "一"),
        vec![policy(25, false)],
    ]);
    to(
        &mut app,
        "s1",
        Update::Older {
            pushes: older,
            more: false,
            failed: None,
        },
    );
    assert_eq!(
        said(&app),
        [
            ("一".to_string(), Some(Level::ReadOnly)),
            ("三".to_string(), Some(Level::Full)),
            ("四".to_string(), Some(Level::ReadOnly)),
        ]
    );
}

#[test]
fn an_older_page_without_a_change_takes_the_level_at_the_top_of_the_newer_one() {
    // 往上翻读进来的那一页没换过级别：照新的那一页开头的级别，不照现在的。
    let (mut app, _sent) = app();
    resume(&mut app, page(vec![turn(30, "三")]), true, Level::Full);
    live_switch(&mut app, true);
    to(
        &mut app,
        "s1",
        Update::Older {
            pushes: page(vec![turn(10, "一")]),
            more: true,
            failed: None,
        },
    );
    let got = levels(&app);
    assert_eq!(got.len(), 4, "两句话、两行收尾：{got:?}");
    assert!(
        got.iter().all(|(_, level)| *level == Some(Level::Full)),
        "{got:?}"
    );
}

#[test]
fn a_turn_that_straddles_the_first_change_ends_with_the_level_it_started_with() {
    // 一轮开始时级别还不知道，中途换了：收尾行照开始时的，读到更早的一页再补，不照换成的那一级。
    let (mut app, _sent) = app();
    let mut events = turn(30, "三");
    events.insert(2, policy(32, false));
    resume(&mut app, page(vec![events]), true, Level::Full);
    let older = page(vec![vec![created(true)], turn(10, "一")]);
    to(
        &mut app,
        "s1",
        Update::Older {
            pushes: older,
            more: false,
            failed: None,
        },
    );
    let read_only = Some(Level::ReadOnly);
    assert!(
        levels(&app).iter().all(|(_, level)| *level == read_only),
        "{:?}",
        levels(&app)
    );
}

#[test]
fn a_new_session_keeps_the_bars_when_the_level_changes() {
    let (mut app, _sent) = app();
    app.talk.transcript.link = crate::transcript::Link::Ready;
    app.talk.transcript.user("一".into(), Vec::new());
    app.core(Update::Ready("s1".into()));
    app.core(snapshot(Level::Full));
    live_switch(&mut app, true);
    app.talk.transcript.user("二".into(), Vec::new());
    live_switch(&mut app, false);
    assert_eq!(
        said(&app),
        [
            ("一".to_string(), Some(Level::Full)),
            ("二".to_string(), Some(Level::ReadOnly)),
        ]
    );
}
