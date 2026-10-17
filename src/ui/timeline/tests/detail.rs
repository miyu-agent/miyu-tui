//! 点开翻记忆、查知识库、读文档：照 `tool.result` 的 `detail` 画给人看的（核心 D-6，2026-10-17 项目主人），没有 `detail` 的
//! （老日志、出错的）照结果的原文。

use std::time::Instant;

use ratatui::style::Modifier;
use serde_json::{Value, json};

use super::{rows, segment, step, text};
use crate::core::ToolStatus;
use crate::transcript::{Step, StepKind, ToolState};
use crate::ui::rows::Target;
use crate::ui::test_support::Fixture;

fn tool(name: &str, output: &str, detail: Option<Value>) -> Step {
    let mut s = step(
        StepKind::Tool {
            name: name.into(),
            args: String::new(),
            parsed: json!({"query": "水果"}),
            state: ToolState::Done(ToolStatus::Ok),
            output: output.into(),
            said: None,
        },
        Instant::now(),
        0,
        1,
    );
    s.detail = detail;
    s.open = Some(true);
    s
}

fn opened(step: Step) -> (Vec<String>, Vec<crate::ui::rows::Row>) {
    let f = Fixture::new();
    let got = rows(0, &segment(vec![step], Some(true)), &f.ctx());
    (text(&got), got)
}

#[test]
fn memory_search_lists_memories_by_date_and_strikes_the_forgotten_ones() {
    let detail = json!({
        "memories": [
            {"id": "m3", "class": "user", "date": "2026-10-17", "text": "最喜欢的水果是芒果"},
            {"id": "m1", "class": "user", "date": "2026-10-01", "text": "最喜欢的水果是苹果", "forgotten": true}
        ],
        "turns": [{"date": "2026-10-12", "session": "019a5c3e-8f2b-7c61-9e0d-2f5a7b3c1d4e", "text": "聊到了荔枝"}]
    });
    let (shown, got) = opened(tool("memory_search", "m3 user 2026-10-17: …", Some(detail)));
    assert!(
        shown
            .iter()
            .any(|l| l.contains("2026-10-17 · 最喜欢的水果是芒果")),
        "{shown:?}"
    );
    let forgotten = got.iter().find(|r| r.plain.contains("苹果")).unwrap();
    assert!(
        forgotten
            .line
            .spans
            .iter()
            .any(|s| s.style.add_modifier.contains(Modifier::CROSSED_OUT)),
        "作废的划线"
    );
    assert!(
        shown.iter().any(|l| l.contains("2026-10-12 · 会话 ")),
        "{shown:?}"
    );
    assert!(shown.iter().any(|l| l.contains("聊到了荔枝")), "{shown:?}");
    assert!(
        shown.iter().all(|l| !l.contains("m3 user")),
        "不画给模型的原文：{shown:?}"
    );
}

#[test]
fn kb_search_shows_each_hit_under_its_place() {
    let detail = json!({"hits": [
        {"kb": "notes", "path": "fruit.md", "heading": "芒果", "snippet": "**芒果**是热带水果"},
        {"kb": "notes", "path": "book.pdf", "page": 2, "snippet": "第二页讲荔枝"}
    ]});
    let (shown, _) = opened(tool("kb_search", "[notes] fruit.md: 芒果", Some(detail)));
    assert!(
        shown.iter().any(|l| l.contains("notes / fruit.md · 芒果")),
        "{shown:?}"
    );
    assert!(
        shown
            .iter()
            .any(|l| l.contains("芒果是热带水果") && !l.contains("**")),
        "{shown:?}"
    );
    assert!(
        shown
            .iter()
            .any(|l| l.contains("notes / book.pdf · 第 2 页")),
        "{shown:?}"
    );
    assert!(shown.iter().all(|l| !l.contains("[notes]")), "{shown:?}");
}

#[test]
fn kb_read_draws_the_page_as_markdown() {
    let detail = json!({"kb": "notes", "path": "fruit.md", "page": 1, "pages": 3,
        "text": "# 热带水果\n\n**芒果**、荔枝"});
    let (shown, _) = opened(tool(
        "kb_read",
        "# 热带水果\n…\n(Page 1 of 3.)",
        Some(detail),
    ));
    assert!(
        shown
            .iter()
            .any(|l| l.contains("热带水果") && !l.contains('#')),
        "标题照 Markdown 画：{shown:?}"
    );
    assert!(
        shown
            .iter()
            .any(|l| l.contains("芒果、荔枝") && !l.contains("**")),
        "{shown:?}"
    );
    assert!(
        shown.iter().all(|l| !l.contains("Page 1 of 3")),
        "{shown:?}"
    );
}

#[test]
fn without_a_detail_the_result_text_is_drawn_as_before() {
    let (shown, _) = opened(tool("memory_search", "Nothing found.", None));
    assert!(
        shown.iter().any(|l| l.contains("Nothing found.")),
        "{shown:?}"
    );
}

#[test]
fn nothing_found_cannot_be_opened() {
    // 没找到：核心给空列表，标题已经写了「没找到」，点开只是一块空的底色。
    let f = Fixture::new();
    let search = |name: &str, detail: Value| {
        let mut s = tool(name, "Nothing found.", Some(detail));
        s.open = None;
        s
    };
    let read = tool("read", "a", None);
    for (name, detail) in [
        ("memory_search", json!({"memories": [], "turns": []})),
        ("kb_search", json!({"hits": []})),
    ] {
        let mut other = read.clone();
        other.open = None;
        let seg = segment(vec![search(name, detail), other], Some(true));
        let got = rows(0, &seg, &f.ctx());
        assert!(
            got.iter().all(|r| r.target != Some(Target::Step(0, 0))),
            "{name} 没找到：点不开"
        );
        assert!(got.iter().any(|r| r.target == Some(Target::Step(0, 1))));
    }
    // 找到了的照常能点。
    let found = search(
        "kb_search",
        json!({"hits": [{"kb": "n", "path": "a.md", "snippet": "x"}]}),
    );
    let seg = segment(vec![found, read], Some(true));
    assert!(
        rows(0, &seg, &f.ctx())
            .iter()
            .any(|r| r.target == Some(Target::Step(0, 0)))
    );
}
