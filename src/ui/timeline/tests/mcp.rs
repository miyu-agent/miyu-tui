//! MCP 服务的工具（核心 X-3：`mcp__<服务>__<工具>`）：核心没给给人看的名字的，写「服务 · 工具」（2026-10-11 项目主人定）。

use std::time::Instant;

use serde_json::json;

use super::{rows, segment, step, text, thought};
use crate::core::ToolStatus;
use crate::transcript::{StepKind, ToolState};
use crate::ui::test_support::Fixture;

#[test]
fn an_mcp_tool_is_named_by_its_service_and_tool() {
    let f = Fixture::new();
    let call = step(
        StepKind::Tool {
            name: "mcp__github__create_issue".into(),
            args: String::new(),
            parsed: json!({"title": "x"}),
            state: ToolState::Done(ToolStatus::Ok),
            output: String::new(),
            said: None,
        },
        Instant::now(),
        0,
        1,
    );
    // 和一步思考放在一段里展开：一步一行写标题。
    let both = segment(vec![thought(Instant::now(), 0), call], Some(true));
    let shown = text(&rows(0, &both, &f.ctx()));
    assert!(
        shown.iter().any(|l| l.contains("github · create_issue")),
        "{shown:?}"
    );
    assert!(shown.iter().all(|l| !l.contains("mcp__")), "{shown:?}");
}

#[test]
fn a_finished_step_is_named_by_what_the_result_recorded_at_call_time() {
    // 核心 9-8 六补：结果里记下调用那一刻的显示名（脚本、MCP、扩展工具），以它为准，照界面语言挑；卸了包、关了服务也不变。
    let f = Fixture::new();
    let mut call = step(
        StepKind::Tool {
            name: "mcp__dice__roll".into(),
            args: String::new(),
            parsed: json!({}),
            state: ToolState::Done(ToolStatus::Ok),
            output: String::new(),
            said: None,
        },
        Instant::now(),
        0,
        1,
    );
    call.display = [("en", "Dice"), ("zh", "掷骰子")]
        .into_iter()
        .map(|(l, n)| (l.to_string(), n.to_string()))
        .collect();
    let both = segment(vec![thought(Instant::now(), 0), call], Some(true));
    let shown = text(&rows(0, &both, &f.ctx()));
    assert!(shown.iter().any(|l| l.contains("掷骰子")), "{shown:?}");
    assert!(
        shown.iter().all(|l| !l.contains("dice · roll")),
        "不照这一刻的表拼：{shown:?}"
    );
}
