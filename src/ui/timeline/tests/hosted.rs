//! 供应商侧工具（模型在供应商那边自己跑的，事件里的 `hosted_tool`，核心 HT-1/HT-3）：标题照核心 `view.md`「供应商侧
//! 工具」写「网络搜索（agy）」（2026-11-16 项目主人定），流出来的、补发来的一样；不抢核心执行的那件的调用编号。

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::{rows, segment, text};
use crate::config::Config;
use crate::core::{Push, ToolStatus, Update};
use crate::transcript::{StepKind, ToolState, Transcript};
use crate::ui::test_support::Fixture;

/// 模型说的一条事件：`ag` 这一家（主人配的名字是 `agy`）。
fn by_model(seq: u64, kind: &str, body: Value) -> Value {
    json!({"seq": seq, "at": "2026-11-16T08:00:00Z", "kind": kind, "turn": 1,
        "by": {"kind": "model", "endpoint": "ag", "model": "gemini"}, "body": body})
}

/// 一次回复：先搜一下（供应商自己跑），再读一个文件（核心执行）。`live` 带流式的增量，不带的只有落了盘的。
fn events(live: bool) -> Vec<Value> {
    let query = "{\"query\":\"rust\"}";
    let mut out = vec![
        json!({"seq": 1, "at": "2026-11-16T08:00:00Z", "kind": "turn.started", "turn": 1,
        "by": {"kind": "kernel"}, "body": {}}),
    ];
    if live {
        out.extend([
            by_model(
                2,
                "model.delta",
                json!({"index": 0, "start": "hosted_tool", "name": "search_web", "seen": 1}),
            ),
            by_model(3, "model.delta", json!({"index": 0, "text": query})),
            by_model(
                4,
                "model.delta",
                json!({"index": 0, "outcome": {"text": "3 results"}}),
            ),
            by_model(5, "model.delta", json!({"index": 0, "end": true})),
            by_model(
                6,
                "model.delta",
                json!({"index": 1, "start": "tool_call", "name": "read"}),
            ),
            by_model(
                7,
                "model.delta",
                json!({"index": 1, "text": "{\"path\":\"src\"}", "end": true}),
            ),
        ]);
    }
    out.push(by_model(
        8,
        "message.assistant",
        json!({"seen": 1, "blocks": [
        {"type": "hosted_tool", "call_id": "h1", "name": "search_web", "args": query,
            "outcome": {"text": "3 results"}},
        {"type": "tool_call", "call_id": "call_2", "name": "read", "args": "{\"path\":\"src\"}"}]}),
    ));
    out.push(json!({"seq": 9, "at": "2026-11-16T08:00:01Z", "kind": "tool.result", "turn": 1,
        "by": {"kind": "tool", "call_id": "call_2"},
        "body": {"call_id": "call_2", "status": "ok", "blocks": [{"type": "text", "text": "lib.rs"}]}}));
    out
}

/// 读成会话，交回第一段的几步。
fn steps(pushes: Vec<Push>) -> Vec<crate::transcript::Step> {
    let texts = Config::builtin().unwrap().text;
    let mut t = Transcript::default();
    for push in pushes {
        t.update(Update::Push(push), &texts);
    }
    let segment = t.entries.iter().find_map(|e| e.segment.as_ref());
    segment.expect("有一段时间线").steps.clone()
}

/// 照 `language` 排出来的标题行；`named` 的记下了供应商 `ag` 的名字是 `agy`。
fn titled(steps: Vec<crate::transcript::Step>, language: &str, named: bool) -> Vec<String> {
    let table = crate::language::LanguageTable::builtin().unwrap();
    let mut f = Fixture::new();
    f.config = Config::load(&table.find(language).unwrap(), false).unwrap();
    f.human = crate::human::Human::from_resources(language);
    if named {
        f.human
            .add_providers(&BTreeMap::from([("ag".to_string(), "agy".to_string())]));
    }
    text(&rows(0, &segment(steps, Some(true)), &f.ctx()))
}

fn titles(steps: Vec<crate::transcript::Step>, language: &str) -> Vec<String> {
    titled(steps, language, true)
}

#[test]
fn a_hosted_search_is_titled_with_the_core_tool_and_who_ran_it() {
    for (how, pushes) in [
        ("流出来的", crate::core::read_live(&events(true))),
        ("补发来的", crate::core::read_replayed(&events(false))),
    ] {
        let steps = steps(pushes);
        assert_eq!(steps.len(), 2, "{how}：{steps:?}");
        let StepKind::Tool { state, output, .. } = &steps[0].kind else {
            panic!("{how}：{steps:?}");
        };
        assert_eq!(state, &ToolState::Done(ToolStatus::Ok), "{how}");
        assert_eq!(output, "3 results", "{how}");
        // 核心执行的那一件拿到自己的调用编号和结果，没被供应商侧那一步抢走。
        assert_eq!(steps[1].call_id.as_deref(), Some("call_2"), "{how}");
        assert!(
            matches!(&steps[1].kind, StepKind::Tool { output, .. } if output == "lib.rs"),
            "{how}"
        );
        let zh = titles(steps.clone(), "zh");
        assert!(
            zh.iter().any(|l| l.contains("网络搜索（agy） · rust")),
            "{how}：{zh:?}"
        );
        assert!(
            zh.iter().all(|l| !l.contains("search_web")),
            "{how}：{zh:?}"
        );
        let ja = titles(steps.clone(), "ja");
        assert!(
            ja.iter().any(|l| l.contains("（agy） · rust")),
            "{how}：{ja:?}"
        );
        let en = titles(steps, "en");
        assert!(
            en.iter().any(|l| l.contains("Web search (agy) · rust")),
            "{how}：{en:?}"
        );
    }
}

/// DeepSeek 官方 Responses 接口自己跑的一次搜索、一次读网页（核心 DSR-4）：参数是 `action` 的原文，搜索一次带几句
/// `queries`。`live` 带流式的增量，不带的只有落了盘的。
fn deepseek_events(live: bool) -> Vec<Value> {
    let by = |seq: u64, kind: &str, body: Value| {
        json!({"seq": seq, "at": "2026-11-19T08:00:00Z", "kind": kind, "turn": 1,
            "by": {"kind": "model", "endpoint": "deepseek", "model": "deepseek-flash"}, "body": body})
    };
    let search = r#"{"type":"search","queries":["rust 1.90 release","rust blog"]}"#;
    let open = r#"{"type":"open_page","url":"https://blog.rust-lang.org/"}"#;
    let mut out = vec![
        json!({"seq": 1, "at": "2026-11-19T08:00:00Z", "kind": "turn.started", "turn": 1,
        "by": {"kind": "kernel"}, "body": {}}),
    ];
    if live {
        for (index, name, args, said) in [
            (
                0,
                "deepseek_web_search",
                search,
                "Rust Blog - https://blog.rust-lang.org/",
            ),
            (1, "deepseek_read_url", open, "https://blog.rust-lang.org/"),
        ] {
            let seq = 2 + index * 4;
            out.extend([
                by(
                    seq,
                    "model.delta",
                    json!({"index": index, "start": "hosted_tool", "name": name}),
                ),
                by(
                    seq + 1,
                    "model.delta",
                    json!({"index": index, "text": args}),
                ),
                by(
                    seq + 2,
                    "model.delta",
                    json!({"index": index, "outcome": {"text": said}}),
                ),
                by(seq + 3, "model.delta", json!({"index": index, "end": true})),
            ]);
        }
    }
    out.push(by(
        10,
        "message.assistant",
        json!({"blocks": [
        {"type": "hosted_tool", "call_id": "ws_1", "name": "deepseek_web_search", "args": search,
            "outcome": {"text": "Rust Blog - https://blog.rust-lang.org/"}},
        {"type": "hosted_tool", "call_id": "ws_2", "name": "deepseek_read_url", "args": open,
            "outcome": {"text": "https://blog.rust-lang.org/"}},
        {"type": "text", "text": "done"}]}),
    ));
    out
}

#[test]
fn deepseek_search_and_page_reads_are_titled_like_the_core_tools() {
    for (how, pushes) in [
        ("流出来的", crate::core::read_live(&deepseek_events(true))),
        (
            "补发来的",
            crate::core::read_replayed(&deepseek_events(false)),
        ),
    ] {
        let steps = steps(pushes);
        let table = crate::language::LanguageTable::builtin().unwrap();
        for (language, search, fetch) in [
            (
                "zh",
                "网络搜索（DeepSeek） · rust 1.90 release",
                "抓取网页（DeepSeek） · https://blog.rust-lang.org/",
            ),
            (
                "en",
                "Web search (DeepSeek) · rust 1.90 release",
                "Fetch page (DeepSeek) · https://blog.rust-lang.org/",
            ),
            (
                "ja",
                "（DeepSeek） · rust 1.90 release",
                "（DeepSeek） · https://blog.rust-lang.org/",
            ),
        ] {
            let mut f = Fixture::new();
            f.config = Config::load(&table.find(language).unwrap(), false).unwrap();
            f.human = crate::human::Human::from_resources(language);
            f.human.add_providers(&BTreeMap::from([(
                "deepseek".to_string(),
                "DeepSeek".to_string(),
            )]));
            let got = text(&rows(0, &segment(steps.clone(), Some(true)), &f.ctx()));
            // 搜索写第一句查的词（同核心 `title.rs` 的 `first_text`），读网页写网址；不露 DeepSeek 自己的工具名。
            assert!(
                got.iter().any(|l| l.contains(search)),
                "{how} {language}：{got:?}"
            );
            assert!(
                got.iter().any(|l| l.contains(fetch)),
                "{how} {language}：{got:?}"
            );
            assert!(
                got.iter().all(|l| !l.contains("deepseek_")),
                "{how} {language}：{got:?}"
            );
        }
    }
}

#[test]
fn an_unknown_hosted_tool_keeps_its_name_and_no_name_means_the_provider_id() {
    let mut events = events(true);
    for event in &mut events {
        if event["body"]["name"] == "search_web" {
            event["body"]["name"] = json!("code_execution");
        }
        if let Some(blocks) = event["body"]["blocks"].as_array_mut() {
            blocks[0]["name"] = json!("code_execution");
        }
    }
    let got = titled(steps(crate::core::read_live(&events)), "en", false);
    assert!(
        got.iter()
            .any(|l| l.contains("code_execution (ag)") && !l.contains("rust")),
        "认不得的写原名、没有对象，没记下名字的照供应商编号：{got:?}"
    );
}

#[test]
fn a_hosted_step_cut_off_without_a_result_is_cancelled() {
    let mut events = events(true);
    events.retain(|e| e["body"]["outcome"].is_null());
    let said = events
        .iter_mut()
        .find(|e| e["kind"] == "message.assistant")
        .unwrap();
    said["body"]["interrupted"] = json!(true);
    said["body"]["blocks"][0]
        .as_object_mut()
        .unwrap()
        .remove("outcome");
    let steps = steps(crate::core::read_live(&events));
    assert!(matches!(
        &steps[0].kind,
        StepKind::Tool {
            state: ToolState::Done(ToolStatus::Cancelled),
            ..
        }
    ));
}
