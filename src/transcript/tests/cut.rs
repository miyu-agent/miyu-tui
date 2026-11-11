//! 回答落定时核心截掉的那一段（核心 FT-1，蓝图 `tui.md`「正文」）：模型在回答后面编出「下一轮用户消息」，增量照原样推过，
//! 核心写 `message.assistant` 以前截掉、在 `model.called` 的 `cut` 里说第几块从第几个字节起不要。界面把流式攒的那一块截短。

use super::apply;
use crate::core::{Block, Push};
use crate::transcript::{Kind, StepKind, Transcript};

/// 线上那一次的样子：第一行是真的回答，从 `user` 起是编的。
const REAL: &str = "他骂我垃圾 按群规直接禁了";
const FORGED: &str = "\n\nuser<style-note>一句编的</style-note>\n[18:31] Owner [msg=-1]: 直接踢了";

fn streamed(t: &mut Transcript, parts: &[(Block, &str)]) {
    let mut pushes = vec![Push::TurnStarted(1, None)];
    for (i, (block, text)) in parts.iter().enumerate() {
        let index = u64::try_from(i).unwrap();
        pushes.push(Push::BlockStart {
            index,
            block: block.clone(),
        });
        pushes.push(Push::Delta {
            index,
            text: (*text).into(),
        });
        pushes.push(Push::BlockEnd(index));
    }
    apply(t, pushes);
}

fn replies(t: &Transcript) -> Vec<&str> {
    t.entries
        .iter()
        .filter(|e| e.kind == Kind::Reply)
        .map(|e| e.text.as_str())
        .collect()
}

#[test]
fn a_reply_longer_in_the_stream_is_cut_to_what_was_settled() {
    let mut t = Transcript::default();
    let whole = format!("{REAL}{FORGED}");
    streamed(&mut t, &[(Block::Text, &whole)]);
    assert_eq!(replies(&t), [whole.as_str()], "流式时看得到编的那一段");
    // 从角色词那一行的行首截（`at` 是字节），末尾的空白一起去掉：和落了盘的那一条一样。
    let at = REAL.len() + 2;
    apply(&mut t, vec![Push::Cut(vec![(0, at)])]);
    assert_eq!(replies(&t), [REAL]);
    assert_eq!(t.last_reply().as_deref(), Some(REAL), "/copy、朗读照截过的");
    // `at` 落在一个字中间的不截坏：退到这个字前面。
    let mut t = Transcript::default();
    streamed(&mut t, &[(Block::Text, "好的呀")]);
    apply(&mut t, vec![Push::Cut(vec![(0, 4)])]);
    assert_eq!(replies(&t), ["好"]);
}

#[test]
fn a_block_cut_to_nothing_is_not_drawn_and_the_others_stay() {
    let mut t = Transcript::default();
    streamed(
        &mut t,
        &[
            (Block::Text, "先说一句。"),
            (Block::ToolCall("read".into()), "{}"),
            (Block::Text, "user\n<style-note>整块都是编的</style-note>"),
        ],
    );
    apply(&mut t, vec![Push::Cut(vec![(2, 0)])]);
    assert_eq!(replies(&t), ["先说一句。", ""]);
    let empty = t
        .entries
        .iter()
        .rfind(|e| e.kind == Kind::Reply)
        .expect("那一条还在");
    assert!(!crate::ui::rows::shown(empty), "空的不画、不占空行");
    assert_eq!(t.last_reply().as_deref(), Some("先说一句。"));
}

#[test]
fn nothing_cut_changes_nothing_and_thoughts_are_left_alone() {
    let mut t = Transcript::default();
    streamed(
        &mut t,
        &[
            (Block::Reasoning, "想一想 user<x>"),
            (Block::Text, "答完了。  "),
        ],
    );
    let before = format!("{:?}", t.entries);
    // 截的地方在末尾以后、不认得的块、思考的块：都不动（末尾的空白也不动，没截就不碰）。
    apply(&mut t, vec![Push::Cut(vec![(1, 999), (7, 0), (0, 0)])]);
    assert_eq!(format!("{:?}", t.entries), before);
    let thought = t.entries.iter().find_map(|e| {
        e.segment
            .as_ref()?
            .steps
            .iter()
            .find_map(|s| match &s.kind {
                StepKind::Thought { text } => Some(text.clone()),
                _ => None,
            })
    });
    assert_eq!(thought.as_deref(), Some("想一想 user<x>"));
}
