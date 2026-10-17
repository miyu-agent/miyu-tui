//! 记忆、知识库的工具（2026-10-17 项目主人）：结果是给模型看的英文（`Saved as m9.`），点开画给人看的——记住画记下的全文，
//! 记进知识库画写进去的，改知识库画差异，忘掉、删文档点不开；收起那一行记住单算一类（`Remembered 1 memory`），搜记忆不用。

use std::time::Instant;

use serde_json::json;

use super::{rows, segment, step, text, thought};
use crate::core::ToolStatus;
use crate::transcript::{Step, StepKind, ToolState};
use crate::ui::rows::Target;
use crate::ui::test_support::Fixture;

fn tool(name: &str, parsed: serde_json::Value, output: &str, start: u64) -> Step {
    step(
        StepKind::Tool {
            name: name.into(),
            args: String::new(),
            parsed,
            state: ToolState::Done(ToolStatus::Ok),
            output: output.into(),
            said: None,
        },
        Instant::now(),
        start,
        1,
    )
}

/// 只有这一步、点开它：画出来的行。
fn opened(step: Step) -> Vec<String> {
    let f = Fixture::new();
    let mut step = step;
    step.open = Some(true);
    text(&rows(0, &segment(vec![step], Some(true)), &f.ctx()))
}

#[test]
fn opening_remember_shows_what_was_remembered_not_the_reply_to_the_model() {
    let got = opened(tool(
        "remember",
        json!({"class": "fact", "text": "shorin 最喜欢的动物是猫。"}),
        "Saved as m9.",
        0,
    ));
    assert!(
        got.iter().any(|l| l.contains("shorin 最喜欢的动物是猫。")),
        "{got:?}"
    );
    assert!(got.iter().all(|l| !l.contains("Saved as")), "{got:?}");
}

#[test]
fn opening_a_knowledge_save_or_edit_shows_the_text_or_the_diff() {
    let saved = opened(tool(
        "kb_save",
        json!({"kb": "notes", "path": "fruit.md", "text": "芒果、荔枝都是热带水果。"}),
        "Saved fruit.md in notes.",
        0,
    ));
    assert!(
        saved.iter().any(|l| l.contains("芒果、荔枝都是热带水果。")),
        "{saved:?}"
    );
    assert!(saved.iter().all(|l| !l.contains("Saved")), "{saved:?}");
    let edited = opened(tool(
        "kb_edit",
        json!({"kb": "notes", "path": "fruit.md", "old": "荔枝", "new": "龙眼"}),
        "Edited fruit.md in notes.",
        0,
    ));
    assert!(edited.iter().any(|l| l.contains("- 荔枝")), "{edited:?}");
    assert!(edited.iter().any(|l| l.contains("+ 龙眼")), "{edited:?}");
    assert!(edited.iter().all(|l| !l.contains("Edited")), "{edited:?}");
}

#[test]
fn forgetting_and_deleting_cannot_be_opened() {
    let f = Fixture::new();
    let step_rows = |got: &[crate::ui::rows::Row], j: usize| {
        got.iter()
            .filter(|r| r.target == Some(Target::Step(0, j)))
            .count()
    };
    for (name, args) in [
        ("forget", json!({"id": "m9"})),
        ("kb_delete", json!({"kb": "notes", "path": "fruit.md"})),
    ] {
        // 和一般的工具同段：它那一步不当成能点的，一般的照旧能点。
        let search = tool("memory_search", json!({"query": "猫"}), "", 1);
        let seg = segment(
            vec![tool(name, args.clone(), "Retired m9.", 0), search],
            Some(true),
        );
        let got = rows(0, &seg, &f.ctx());
        assert_eq!(step_rows(&got, 0), 0, "{name} 点不开");
        assert!(step_rows(&got, 1) > 0, "一般的工具照旧能点");
        // 只有它一步的一段，点开整段：露出它的标题那一行，不铺一块空的。
        let seg = segment(vec![tool(name, args, "Retired m9.", 0)], Some(true));
        let got = text(&rows(0, &seg, &f.ctx()));
        assert!(got.iter().any(|l| l.contains(name)), "{name}：{got:?}");
        assert!(got.iter().all(|l| !l.contains("Retired")), "{got:?}");
    }
}

#[test]
fn a_folded_segment_counts_remembering_on_its_own() {
    let f = Fixture::new();
    let t0 = Instant::now();
    let fold = |steps| text(&rows(0, &segment(steps, None), &f.ctx()));
    let remember = |start| tool("remember", json!({"text": "猫"}), "Saved as m9.", start);
    let search = |start| tool("memory_search", json!({"query": "猫"}), "", start);
    assert_eq!(
        fold(vec![thought(t0, 0), remember(1)]),
        ["  Remembered 1 memory · 1 thought · 2s"]
    );
    assert_eq!(
        fold(vec![remember(0), remember(1)]),
        ["  Remembered 2 memories · 2s"]
    );
    assert_eq!(
        fold(vec![remember(0), search(1)]),
        ["  Remembered 1 memory · 1 tool · 2s"],
        "搜记忆不单算（项目主人：「搜搜记忆就不用了」）"
    );
    assert_eq!(fold(vec![search(0)]), ["  Used 1 tool · 1s"]);
}

#[test]
fn the_remember_title_keeps_its_result_line() {
    // 记住单算一类以后，标题照旧接结果那一句（「记住 · 内容 · 记成 m9」）。
    let mut f = Fixture::new();
    f.human = crate::human::Human::from_resources("zh");
    let mut step = tool("remember", json!({"text": "猫"}), "Saved as m9.", 0);
    if let StepKind::Tool { said, .. } = &mut step.kind {
        *said = Some(miyu_client::protocol::Said {
            key: "software/memory/remember/saved".into(),
            fields: [("id".to_string(), "m9".to_string())].into_iter().collect(),
        });
    }
    let search = tool("memory_search", json!({"query": "猫"}), "", 1);
    let seg = segment(vec![step, search], Some(true));
    let got = text(&rows(0, &seg, &f.ctx()));
    assert!(
        got.iter().any(|l| l.contains("记住 · 猫 · 记成 m9")),
        "{got:?}"
    );
}
