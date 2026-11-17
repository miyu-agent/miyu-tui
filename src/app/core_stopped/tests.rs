//! 核心是人停的（核心 CS-5）：画「核心已停止」，空着按 `Enter`、敲了话才拉起；别的连不上照旧不拉。

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::core::{Command, StartRefusal, Update};
use crate::transcript::Link;

fn enter(app: &mut crate::app::App) {
    app.handle(Event::Key(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::NONE,
    )));
}

/// 核心交来的「人停的」：`core/start/stopped_by_user`，原话「核心已停止」。
fn stopped() -> Update {
    Update::StartRefused(StartRefusal {
        said: miyu_client::protocol::Said::new("core/start/stopped_by_user"),
        reason: "核心已停止".into(),
        bin: None,
    })
}

#[test]
fn a_core_a_person_stopped_shows_stopped_and_enter_starts_it() {
    let (mut app, _sent, mut starts) = crate::app::test_support::app_with_starts();
    app.core(stopped());
    assert!(
        matches!(&app.talk.transcript.link, Link::Stopped(said) if !said.is_empty()),
        "{:?}",
        app.talk.transcript.link
    );
    assert!(starts.try_recv().is_err(), "没人点不拉");
    enter(&mut app);
    assert!(starts.try_recv().is_ok(), "空着按 Enter 拉起");
    assert_eq!(app.talk.transcript.link, Link::Connecting);
}

#[test]
fn a_sentence_typed_while_stopped_starts_the_core_and_waits_to_be_sent() {
    let (mut app, mut sent, mut starts) = crate::app::test_support::app_with_starts();
    app.talk.transcript.session = Some("s1".into());
    app.core(stopped());
    app.paste("你好");
    enter(&mut app);
    assert!(starts.try_recv().is_ok(), "敲了话就是要用");
    let mut queued = false;
    while let Ok(command) = sent.try_recv() {
        queued |= matches!(command, Command::Send { ref text, .. } if text == "你好");
    }
    assert!(queued, "这一句排着，连上再发");
}

#[test]
fn other_reasons_for_being_down_do_not_start_anything() {
    let (mut app, _sent, mut starts) = crate::app::test_support::app_with_starts();
    app.core(Update::NoCoreBin);
    enter(&mut app);
    app.paste("你好");
    enter(&mut app);
    assert!(
        starts.try_recv().is_err(),
        "核心没在跑、没说怎么拉起的照旧不拉"
    );
    assert!(matches!(app.talk.transcript.link, Link::Down(_)));
}
