//! 联网的两件（2026-10-19 项目主人：一次搜索、一次抓取收起来是「Used 2 tools」）：收起那一行网络搜索、抓取网页各算一类。

use std::time::Instant;

use serde_json::json;

use super::{rows, segment, step, text, thought};
use crate::core::ToolStatus;
use crate::transcript::{Step, StepKind, ToolState};
use crate::ui::test_support::Fixture;

fn tool(name: &str, status: ToolStatus, start: u64) -> Step {
    step(
        StepKind::Tool {
            name: name.into(),
            args: String::new(),
            parsed: json!({"query": "rust", "url": "https://a.example"}),
            state: ToolState::Done(status),
            output: "ok".into(),
            said: None,
        },
        Instant::now(),
        start,
        1,
    )
}

#[test]
fn a_folded_segment_counts_searches_and_fetched_pages_on_their_own() {
    let f = Fixture::new();
    let t0 = Instant::now();
    let fold = |steps| text(&rows(0, &segment(steps, None), &f.ctx()));
    let search = |start| tool("web_search", ToolStatus::Ok, start);
    let fetch = |start| tool("web_fetch", ToolStatus::Ok, start);
    assert_eq!(
        fold(vec![thought(t0, 0), search(1), fetch(2)]),
        ["  Searched the web 1 time · 1 page · 1 thought · 3s"]
    );
    assert_eq!(
        fold(vec![search(0), search(1)]),
        ["  Searched the web 2 times · 2s"]
    );
    assert_eq!(fold(vec![fetch(0), fetch(1)]), ["  Fetched 2 pages · 2s"]);
    let other = tool("grep", ToolStatus::Ok, 2);
    assert_eq!(
        fold(vec![search(0), fetch(1), other]),
        ["  Searched the web 1 time · 1 page · 1 tool · 3s"],
        "别的工具照旧算 tools"
    );
    // 出错的没搜成：算工具（同记住）。
    assert_eq!(
        fold(vec![tool("web_search", ToolStatus::Error, 0)]),
        ["  Used 1 tool · 1 err · 1s"]
    );
}

#[test]
fn the_search_title_keeps_its_result_line() {
    // 单算一类以后，标题照旧接结果那一句（「网络搜索 · rust · 8 条（Exa）」）。
    let mut f = Fixture::new();
    f.human = crate::human::Human::from_resources("zh");
    let mut search = tool("web_search", ToolStatus::Ok, 0);
    if let StepKind::Tool { said, .. } = &mut search.kind {
        *said = Some(miyu_client::protocol::Said {
            key: "software/net/web_search/found".into(),
            fields: [("hits", "8"), ("source", "Exa")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        });
    }
    let fetch = tool("web_fetch", ToolStatus::Ok, 1);
    let got = text(&rows(
        0,
        &segment(vec![search, fetch], Some(true)),
        &f.ctx(),
    ));
    assert!(got.iter().any(|l| l.contains("rust · 8 条")), "{got:?}");
}
