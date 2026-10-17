//! 中途接进一个正在跑的会话（2026-10-17 项目主人定）：「工作区：…」那一句插在正在跑的这一轮上面，这一轮不收起，之后的
//! 增量照旧落到它自己那一条上。

use super::super::Transcript;
use super::{Kind, apply};
use crate::config::Config;
use crate::core::{Block, Push};

#[test]
fn the_workspace_note_goes_above_a_running_turn_and_its_blocks_keep_their_place() {
    let texts = Config::builtin().unwrap().text;
    let mut t = Transcript::default();
    t.user("看看笔记".into(), Vec::new());
    apply(
        &mut t,
        vec![
            Push::UserMessage(1),
            Push::TurnStarted(2, Some(1)),
            Push::BlockStart {
                index: 0,
                block: Block::Reasoning,
            },
            Push::Delta {
                index: 0,
                text: "先想".into(),
            },
        ],
    );
    t.joined_workspace("/elsewhere".into(), true, "/here", &texts);
    let kinds: Vec<Kind> = t.entries.iter().map(|e| e.kind.clone()).collect();
    assert_eq!(
        kinds,
        [Kind::Note, Kind::User, Kind::Steps],
        "插在这一轮上面"
    );
    assert_eq!(t.folds(), 0, "正在想的那一段不收起");
    // 之后的增量落到原来那一段，不落到别处。
    apply(
        &mut t,
        vec![Push::Delta {
            index: 0,
            text: "再想".into(),
        }],
    );
    let thought = format!("{:?}", t.entries[2].segment);
    assert!(thought.contains("先想再想"), "{thought}");
}
