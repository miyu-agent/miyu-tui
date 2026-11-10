//! 会话读不了的两种（`unread.rs`，核心 YC-2、YC-6）。

use std::time::Instant;

use super::app;
use crate::app::unread::{TRIES, WAIT};
use crate::core::{Command, Update};

/// 交出去的命令里有几条再试，最后一条是什么。
fn rereads(sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    while let Ok(command) = sent.try_recv() {
        if let Command::Reread { session, page } = command {
            out.push((session, page));
        }
    }
    out
}

#[test]
fn a_session_that_cannot_be_read_for_now_is_tried_again_and_then_given_up() {
    let (mut app, mut sent) = app();
    app.talk.transcript.session = Some("s-one".into());
    app.core(Update::SessionUnavailable {
        session: "s-one".into(),
        page: false,
    });
    let notice = app
        .typing
        .notice
        .as_ref()
        .map(|n| n.text.clone())
        .unwrap_or_default();
    assert_eq!(notice, "会话暂时无法读取，5 秒后自动重试");
    assert!(app.talk.transcript.session.is_some(), "不当成会话没了");
    let now = Instant::now();
    app.reread_due(now);
    assert!(rereads(&mut sent).is_empty(), "没到点不试");
    app.reread_due(now + WAIT);
    assert_eq!(
        rereads(&mut sent),
        [("s-one".to_string(), false)],
        "订阅没成的只再订阅"
    );
    app.reread_due(now + WAIT * 3);
    assert!(rereads(&mut sent).is_empty(), "等着核心回，不再发");
    // 再试又读不了：接着数，到了次数不再试
    for _ in 1..TRIES {
        app.core(Update::SessionUnavailable {
            session: "s-one".into(),
            page: false,
        });
        app.reread_due(Instant::now() + WAIT);
        assert_eq!(rereads(&mut sent).len(), 1);
    }
    app.core(Update::SessionUnavailable {
        session: "s-one".into(),
        page: false,
    });
    app.reread_due(Instant::now() + WAIT * 2);
    assert!(rereads(&mut sent).is_empty(), "试满 {TRIES} 次不再试");
    let notice = app
        .typing
        .notice
        .as_ref()
        .map(|n| n.text.clone())
        .unwrap_or_default();
    assert_eq!(notice, "会话暂时无法读取，请稍后重试");
}

#[test]
fn a_page_that_could_not_be_read_is_read_again_and_a_session_switched_away_is_not() {
    let (mut app, mut sent) = app();
    app.talk.transcript.session = Some("s-one".into());
    app.core(Update::SessionUnavailable {
        session: "s-one".into(),
        page: true,
    });
    app.reread_due(Instant::now() + WAIT);
    assert_eq!(
        rereads(&mut sent),
        [("s-one".to_string(), true)],
        "最新一页没读成的从读页起"
    );
    // 切走了的不再试
    app.core(Update::SessionUnavailable {
        session: "s-one".into(),
        page: true,
    });
    app.talk.transcript.session = Some("s-two".into());
    app.reread_due(Instant::now() + WAIT);
    assert!(rereads(&mut sent).is_empty());
}

#[test]
fn a_session_broken_in_the_middle_says_where_and_stays_view_only() {
    let (mut app, _sent) = app();
    app.talk.transcript.session = Some("s-one".into());
    app.core(Update::SessionReadOnly {
        session: "s-one".into(),
        line: Some(7),
    });
    assert!(app.talk.transcript.read_only);
    assert_eq!(
        app.talk.transcript.session.as_deref(),
        Some("s-one"),
        "照旧对着它"
    );
    let said: Vec<_> = app
        .talk
        .transcript
        .entries
        .iter()
        .map(|e| e.text.clone())
        .collect();
    let line = "会话日志第 7 行坏了，只能看到这一行以前的内容，不能接着聊。要继续请新开一个会话。";
    assert_eq!(
        said.iter().filter(|t| t.as_str() == line).count(),
        1,
        "{said:?}"
    );
    // 重连又拒一次：不再写第二遍
    app.core(Update::SessionReadOnly {
        session: "s-one".into(),
        line: Some(7),
    });
    let said: Vec<_> = app
        .talk
        .transcript
        .entries
        .iter()
        .map(|e| e.text.clone())
        .collect();
    assert_eq!(said.iter().filter(|t| t.as_str() == line).count(), 1);
}
