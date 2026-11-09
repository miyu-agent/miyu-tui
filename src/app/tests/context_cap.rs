//! 会话的上下文上限（核心 CTX-1、CTX-2，设计 68，CTX-3）：带数值的 `/context` 交核心，不带的照旧开明细；推来上限换了，
//! 照 `session.context` 重读生效的窗口、压缩线。

use serde_json::json;

use crate::core::{Command, Push, Update};
use crate::input::Draft;

/// 发给核心的命令。
fn drained(sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>) -> Vec<Command> {
    std::iter::from_fn(|| sent.try_recv().ok()).collect()
}

#[test]
fn context_with_a_number_goes_to_the_core_and_without_opens_the_box() {
    let (mut app, mut sent) = crate::app::test_support::app();
    app.talk.transcript.session = Some("s1".into());
    drained(&mut sent);
    app.submit(Draft::plain("/context 128k"));
    let slashes: Vec<String> = drained(&mut sent)
        .into_iter()
        .filter_map(|c| match c {
            Command::Slash(line) => Some(line),
            _ => None,
        })
        .collect();
    assert_eq!(slashes, ["/context 128k"], "原样交核心");
    assert!(app.boxes.context.is_none(), "不开明细");
    app.submit(Draft::plain("/context"));
    assert!(app.boxes.context.is_some(), "不带数值照旧开明细");
}

#[test]
fn a_changed_cap_reads_the_window_again() {
    let (mut app, mut sent) = crate::app::test_support::app();
    app.talk.transcript.session = Some("s1".into());
    app.talk.transcript.limits.window = Some(1_000_000);
    drained(&mut sent);
    app.core(Update::Elsewhere {
        session: "s1".into(),
        update: Box::new(Update::Push(Push::ContextSet(Some(128_000)))),
    });
    let tag = drained(&mut sent)
        .into_iter()
        .find_map(|c| match c {
            Command::Ask {
                tag,
                method: "session.context",
                ..
            } => Some(tag),
            _ => None,
        })
        .expect("重读一次");
    app.core(Update::Answer {
        tag,
        result: Ok(json!({"window": 128_000, "line": 101_000, "context": 128_000, "parts": []})),
    });
    let limits = app.talk.transcript.limits;
    assert_eq!(
        (limits.window, limits.compaction_line, limits.context),
        (Some(128_000), Some(101_000), Some(128_000))
    );
}
