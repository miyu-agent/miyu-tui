//! `/usage` 开框时问看着的会话的套餐额度（核心 QU-1，2026-11-14 项目主人定样子）。

use serde_json::json;

use super::app;
use crate::core::{Command, Update};

/// 开框以后发出去的那一次 `session.quota`：编号和参数。
fn asked(
    sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>,
) -> Option<(u64, serde_json::Value)> {
    std::iter::from_fn(|| sent.try_recv().ok()).find_map(|command| match command {
        Command::Ask {
            tag,
            method: "session.quota",
            params,
            ..
        } => Some((tag, params)),
        _ => None,
    })
}

#[test]
fn opening_usage_asks_the_quota_and_keeps_what_comes_back() {
    let (mut app, mut sent) = app();
    app.talk.transcript.session = Some("s1".into());
    app.open_usage();
    let (tag, params) = asked(&mut sent).expect("开框时问");
    assert_eq!(params, json!({"session": "s1"}));
    app.core(Update::Answer {
        tag,
        result: Ok(json!({"windows": [{"id": "rolling", "label": "5 小时", "percent": 4, "status": "ok"}]})),
    });
    let quota = &app.boxes.usage.as_ref().unwrap().quota;
    assert_eq!(quota.len(), 1);
    assert_eq!(quota[0].label, "5 小时");
}

#[test]
fn no_session_no_question_and_empty_answer_draws_nothing() {
    let (mut app, mut sent) = app();
    app.open_usage();
    assert!(asked(&mut sent).is_none(), "还没开会话的不问");
    app.talk.transcript.session = Some("s1".into());
    app.open_usage();
    let (tag, _) = asked(&mut sent).unwrap();
    app.core(Update::Answer {
        tag,
        result: Ok(json!({})),
    });
    assert!(app.boxes.usage.as_ref().unwrap().quota.is_empty());
}
