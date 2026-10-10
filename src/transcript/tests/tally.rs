//! 收起那一行照哪条命令的短标题写（蓝图 `tui.md`「时间线」第 17 条）：一段里只有一条命令的照它的短标题，思考、别的工具、
//! 编辑在旁边也照它，标题照命令自己的参数取。从 `mod.rs` 挪出来（2026-10-11：过了 500 行）。

use super::super::{Tally, Transcript};
use super::apply;
use crate::core::{Block, Push};

#[test]
fn the_tally_names_a_lone_command_by_its_title() {
    let mut t = Transcript::default();
    apply(
        &mut t,
        vec![
            Push::TurnStarted(1, None),
            Push::BlockStart {
                index: 0,
                block: Block::ToolCall("shell".into()),
            },
            Push::Delta {
                index: 0,
                text: "{\"command\":\"ls\",\"description\":\"列目录\"}".into(),
            },
            Push::BlockEnd(0),
        ],
    );
    let kind_of = |name: &str| (name == "shell").then_some(crate::config::ToolKind::Command);
    let tally = Tally::count(t.entries[0].segment.as_ref().unwrap(), kind_of);
    assert_eq!(tally.command_title.as_deref(), Some("列目录"));
    assert_eq!((tally.commands, tally.thoughts), (1, 0));
}

#[test]
fn thoughts_do_not_stop_a_lone_command_from_naming_the_segment() {
    let mut t = Transcript::default();
    apply(
        &mut t,
        vec![
            Push::TurnStarted(1, None),
            Push::BlockStart {
                index: 0,
                block: Block::Reasoning,
            },
            Push::Delta {
                index: 0,
                text: "先看看目录".into(),
            },
            Push::BlockStart {
                index: 1,
                block: Block::ToolCall("shell".into()),
            },
            Push::Delta {
                index: 1,
                text: "{\"command\":\"ls\",\"description\":\"列目录\"}".into(),
            },
            Push::BlockEnd(1),
        ],
    );
    let kind_of = |name: &str| (name == "shell").then_some(crate::config::ToolKind::Command);
    let tally = Tally::count(t.entries[0].segment.as_ref().unwrap(), kind_of);
    assert_eq!((tally.commands, tally.thoughts), (1, 1));
    assert_eq!(tally.command_title.as_deref(), Some("列目录"));
}

#[test]
fn a_command_names_the_segment_even_beside_other_tools() {
    // 2026-10-02 项目主人定：一段里只有一条命令、它有短标题，和编辑、别的工具同段也用短标题打头。
    let mut t = Transcript::default();
    apply(
        &mut t,
        vec![
            Push::TurnStarted(1, None),
            Push::BlockStart {
                index: 0,
                block: Block::ToolCall("shell".into()),
            },
            Push::Delta {
                index: 0,
                text: "{\"command\":\"ls\",\"description\":\"列目录\"}".into(),
            },
            Push::BlockEnd(0),
            Push::BlockStart {
                index: 1,
                block: Block::ToolCall("edit".into()),
            },
            Push::Delta {
                index: 1,
                text: "{\"file_path\":\"a.rs\",\"edits\":[]}".into(),
            },
            Push::BlockEnd(1),
        ],
    );
    let kind_of = |name: &str| match name {
        "shell" => Some(crate::config::ToolKind::Command),
        "edit" => Some(crate::config::ToolKind::Edit),
        _ => None,
    };
    let tally = Tally::count(t.entries[0].segment.as_ref().unwrap(), kind_of);
    assert_eq!(tally.command_title.as_deref(), Some("列目录"));
    assert!(!tally.lone, "还编辑了一处，不是只有一件");
}

#[test]
fn the_command_title_comes_from_the_command_not_another_tool() {
    // 同段里还有派子代理（它也带 `description`）：短标题要取命令那一步的，不取先出现的别的工具。
    let mut t = Transcript::default();
    apply(
        &mut t,
        vec![
            Push::TurnStarted(1, None),
            Push::BlockStart {
                index: 0,
                block: Block::ToolCall("subagent".into()),
            },
            Push::Delta {
                index: 0,
                text: "{\"description\":\"查文档\",\"prompt\":\"看看\"}".into(),
            },
            Push::BlockEnd(0),
            Push::BlockStart {
                index: 1,
                block: Block::ToolCall("shell".into()),
            },
            Push::Delta {
                index: 1,
                text: "{\"command\":\"ls\",\"description\":\"列目录\"}".into(),
            },
            Push::BlockEnd(1),
        ],
    );
    let kind_of = |name: &str| match name {
        "shell" => Some(crate::config::ToolKind::Command),
        "subagent" => Some(crate::config::ToolKind::Agent),
        _ => None,
    };
    let tally = Tally::count(t.entries[0].segment.as_ref().unwrap(), kind_of);
    assert_eq!(tally.command_title.as_deref(), Some("列目录"));
}
