use crate::core::{Push, Update};

use super::Turn;

fn said(seq: u64, text: &str) -> Update {
    Update::Push(Push::Said {
        seq,
        text: text.into(),
        attached: Vec::new(),
    })
}

#[test]
fn only_the_turn_this_sentence_started_counts() {
    // 接上的会话先补发最新一页：以前同一句话开的、别的话开的那几轮都不算（图纸「say」第 3 条）。
    let mut turn = Turn::new("你好".into(), None);
    turn.watch(&said(2, "别的话"));
    turn.watch(&Update::Push(Push::TurnStarted(3, Some(2))));
    assert_eq!(turn.ours, None, "别的话开的那一轮");
    turn.watch(&said(9, "你好"));
    turn.watch(&Update::Push(Push::TurnStarted(10, Some(8))));
    assert_eq!(turn.ours, None, "trigger 不是这句话");
    turn.watch(&Update::Push(Push::TurnStarted(11, Some(9))));
    assert_eq!(turn.ours, Some(11));
}

#[test]
fn the_sentence_waits_for_the_terminal_session_before_it_is_sent() {
    let turn = Turn::new("你好".into(), Some("/srv".into()));
    assert_eq!(
        turn.pending.as_deref(),
        Some("你好"),
        "问到终端会话、接上以后才发"
    );
    assert!(!turn.done);
}
