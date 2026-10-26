//! 一轮结束照核心换好的金额换掉这个会话的账（`bill_refresh.rs`，核心 UC-1）。

use serde_json::json;

use super::app;
use crate::core::{Bill, Command, EndReason, Push, Update};

/// 核心推来的一轮结束：照真的形状，外面包着是哪个会话的（`core/take.rs`）。
fn ended(session: &str) -> Update {
    Update::Elsewhere {
        session: session.into(),
        update: Box::new(Update::Push(Push::TurnEnded(EndReason::Completed))),
    }
}

/// 发给核心的命令里那一次 `usage.query`：编号和参数。
fn asked(
    sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>,
) -> Option<(u64, serde_json::Value)> {
    std::iter::from_fn(|| sent.try_recv().ok()).find_map(|command| match command {
        Command::Ask {
            tag,
            method: "usage.query",
            params,
            ..
        } => Some((tag, params)),
        _ => None,
    })
}

fn row(amount: f64) -> serde_json::Value {
    json!({"rows": [{"requests": 3, "usage": {}, "amounts": [{"currency": "USD", "amount": 1.4}],
        "converted": {"currency": "CNY", "amount": amount}}]})
}

/// 这边一笔笔加的原币种：和 [`row`] 的 `amounts` 一样。
fn live(converted: serde_json::Value) -> Bill {
    let mut bill = Bill::read(&json!({ "converted": converted }));
    bill.amounts = Bill::read(&json!({"amounts": [{"currency": "USD", "amount": 1.4}]})).amounts;
    bill
}

#[test]
fn a_finished_turn_asks_for_the_converted_total_and_takes_it() {
    let (mut app, mut sent) = app();
    app.talk.transcript.session = Some("s1".into());
    // 实时来的另一种币先接在后面。
    app.talk.transcript.bill = live(json!({"currency": "CNY", "amount": 7.0,
        "rest": [{"currency": "USD", "amount": 0.4}]}));
    app.core(ended("s1"));
    let (tag, params) = asked(&mut sent).expect("一轮结束问一次");
    assert_eq!(params, json!({"session": "s1"}));
    app.core(Update::Answer {
        tag,
        result: Ok(row(9.8)),
    });
    let converted = app.talk.transcript.bill.converted.clone().expect("换好的");
    assert!((converted.total.amount - 9.8).abs() < 1e-12);
    assert!(converted.rest.is_empty(), "又是一个数");
}

#[test]
fn an_answer_after_the_next_turn_started_or_for_another_session_is_dropped() {
    let (mut app, mut sent) = app();
    app.talk.transcript.session = Some("s1".into());
    let before = live(json!({"currency": "CNY", "amount": 7.0}));
    app.talk.transcript.bill = before.clone();
    app.core(ended("s1"));
    let (tag, _) = asked(&mut sent).expect("问了");
    // 问完又开了一轮：实时的照加，不拿旧的回应盖掉。
    app.talk.transcript.running = Some(std::time::Instant::now());
    app.core(Update::Answer {
        tag,
        result: Ok(row(9.8)),
    });
    assert_eq!(app.talk.transcript.bill, before);
    // 换了会话：不是这个会话的不要。
    app.talk.transcript.running = None;
    app.core(ended("s1"));
    let (tag, _) = asked(&mut sent).expect("问了");
    app.talk.transcript.session = Some("s2".into());
    app.core(Update::Answer {
        tag,
        result: Ok(row(9.8)),
    });
    assert_eq!(app.talk.transcript.bill, before);
    // 核心的账本还没记上这一轮（交回的原币种比这边的少）：不换，不把这一轮的钱丢了。
    app.talk.transcript.session = Some("s1".into());
    app.core(ended("s1"));
    let (tag, _) = asked(&mut sent).expect("问了");
    app.core(Update::Answer {
        tag,
        result: Ok(json!({"rows": [{"requests": 1, "usage": {}, "amounts": [], "unpriced": 1}]})),
    });
    assert_eq!(app.talk.transcript.bill, before, "对不上的照旧用这边的");
}

#[test]
fn money_that_arrives_outside_a_turn_asks_again_and_money_inside_one_does_not() {
    // 起标题、回顾这类旁路请求在一轮结束以后才到：再问一次，又是一个数。一轮进行中的不问，等这一轮结束。
    let (mut app, mut sent) = app();
    app.talk.transcript.session = Some("s1".into());
    let billed = || Update::Elsewhere {
        session: "s1".into(),
        update: Box::new(Update::Push(Push::Billed(crate::core::Cost::read(
            &json!({"currency": "CNY", "amount": 0.0037}),
        )))),
    };
    app.core(billed());
    assert!(asked(&mut sent).is_some(), "一轮外来的钱：问");
    app.talk.transcript.running = Some(std::time::Instant::now());
    app.core(billed());
    assert!(asked(&mut sent).is_none(), "一轮进行中：不问");
}
