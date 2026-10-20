use crate::core::{Push, Update};

use super::Turn;
use crate::shellhook::say::look::Look;

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
    let mut turn = Turn::new("你好".into(), None, Look::new(None));
    // 发出去以前推来的（补发的那一页里同样的话）不认。
    turn.watch(&said(1, "你好"));
    turn.watch(&Update::Push(Push::TurnStarted(1, Some(1))));
    assert_eq!(turn.ours, None, "发出去以前的");
    turn.sent = true;
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
fn a_sentence_sent_on_this_connection_comes_back_as_a_bare_sequence() {
    // 接上已有的会话再发：自己发的那句推回来是 `user_message`，不带字（真模型实测，2026-10-20）。
    let mut turn = Turn::new("你好".into(), None, Look::new(None));
    turn.sent = true;
    turn.watch(&Update::Push(Push::UserMessage(32)));
    turn.watch(&Update::Push(Push::TurnStarted(33, Some(32))));
    assert_eq!(turn.ours, Some(33));
}

#[test]
fn the_sentence_waits_for_the_terminal_session_before_it_is_sent() {
    let turn = Turn::new("你好".into(), Some("/srv".into()), Look::new(None));
    assert_eq!(
        turn.pending.as_deref(),
        Some("你好"),
        "问到终端会话、接上以后才发"
    );
    assert!(!turn.done);
}

#[test]
fn a_sentence_heard_by_a_running_turn_belongs_to_that_turn() {
    // 回报叫醒她的那一轮还在跑的时候说话：这句话排着，被那一轮听到了，不另开一轮（2026-10-20 项目主人验收：终端没还回来）。
    let mut turn = Turn::new("起一个后台任务".into(), None, Look::new(None));
    // 那一轮在补发的那一页里开的、还没结束：发出去以前就记着。
    turn.watch(&Update::Push(Push::TurnStarted(319, Some(315))));
    turn.sent = true;
    turn.watch(&Update::Push(Push::UserMessage(326)));
    assert_eq!(turn.ours, None);
    turn.watch(&Update::Push(Push::Heard(320)));
    assert_eq!(turn.ours, None, "听到的还不到这一句");
    turn.watch(&Update::Push(Push::Heard(326)));
    assert_eq!(turn.ours, Some(319));
}

#[test]
fn the_turn_woken_right_after_ours_is_followed_but_not_one_someone_else_started() {
    // 后台命令在这一轮里跑完、她没顾上看：核心在这一轮结束的同一刻叫醒她再开一轮，接着印（2026-10-20 实测：没人印）。
    let mut turn = Turn::new("后台跑 sleep 3".into(), None, Look::new(None));
    turn.sent = true;
    turn.watch(&Update::Push(Push::UserMessage(367)));
    turn.watch(&Update::Push(Push::TurnStarted(368, Some(367))));
    turn.watch(&Update::Push(Push::TurnEnded(
        crate::core::EndReason::Completed,
    )));
    turn.lingering = true;
    // 别处有人说了话开的那一轮不接。
    turn.watch(&said(380, "别处说的"));
    turn.watch(&Update::Push(Push::TurnStarted(381, Some(380))));
    assert!(turn.lingering, "人说话开的那一轮");
    turn.watch(&Update::Push(Push::TurnEnded(
        crate::core::EndReason::Completed,
    )));
    turn.watch(&Update::Push(Push::TurnStarted(383, Some(379))));
    assert!(!turn.lingering);
    assert_eq!(turn.woken, Some(383));
    assert_eq!(turn.ours, Some(368), "从这句话开的那一轮印起");
    turn.ended = None;
    turn.watch(&Update::Push(Push::TurnEnded(
        crate::core::EndReason::Completed,
    )));
    assert_eq!(turn.ended, Some(crate::core::EndReason::Completed));
}
