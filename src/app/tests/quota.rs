//! `/usage` 开框时问看着的会话的套餐额度（核心 QU-1，2026-11-14 项目主人定样子）、账户余额（核心 QU-2）。

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

#[test]
fn a_balance_answer_is_kept_and_absent_means_none() {
    // 用 DeepSeek 官方供应商的回应是余额（核心 QU-2）：金额照原样的字符串，没带这一格的不画。
    let (mut app, mut sent) = app();
    app.talk.transcript.session = Some("s1".into());
    app.open_usage();
    let (tag, _) = asked(&mut sent).unwrap();
    app.core(Update::Answer {
        tag,
        result: Ok(json!({"balance": {"available": true, "items": [
            {"currency": "CNY", "total": "12.34", "granted": "2.00", "topped_up": "10.34"}]}})),
    });
    let balance = app.boxes.usage.as_ref().unwrap().balance.clone().unwrap();
    assert!(balance.available);
    assert_eq!(balance.items[0].currency, "CNY");
    assert_eq!(balance.items[0].total, "12.34");
    assert_eq!(balance.items[0].granted, "2.00");
    app.open_usage();
    let (tag, _) = asked(&mut sent).unwrap();
    app.core(Update::Answer {
        tag,
        result: Ok(json!({"balance": {"available": false, "items": []}})),
    });
    let balance = app.boxes.usage.as_ref().unwrap().balance.clone().unwrap();
    assert!(!balance.available && balance.items.is_empty());
    app.open_usage();
    let (tag, _) = asked(&mut sent).unwrap();
    app.core(Update::Answer {
        tag,
        result: Ok(json!({})),
    });
    assert_eq!(app.boxes.usage.as_ref().unwrap().balance, None);
}

#[test]
fn the_quota_is_named_after_the_provider_the_session_requests_next() {
    // 2026-11-17 验收第 36 条：标题、余额那一行照供应商的名字（配置起的名字，没起的照目录的），不写死。
    let (mut app, mut sent) = app();
    app.talk.transcript.session = Some("s1".into());
    app.talk.transcript.model = Some(("deepseek-v4.1-flash".into(), "opencode-go".into()));
    let names = [("opencode-go".to_string(), "OpenCode Go".to_string())];
    app.link.human.add_providers(&names.into_iter().collect());
    app.open_usage();
    let (tag, _) = asked(&mut sent).unwrap();
    app.core(Update::Answer {
        tag,
        result: Ok(json!({"windows": [{"id": "rolling", "label": "5 小时", "percent": 4, "status": "ok"}]})),
    });
    let data = app.boxes.usage.as_ref().unwrap();
    assert_eq!(data.quota_from.as_deref(), Some("opencode-go"));
    let seen = app.seen();
    assert_eq!(crate::ui::usage::Sources::of(&seen).provider, "OpenCode Go");
}
