//! 模型输出的一段增量（`model.delta`）：一块开始、接着来的字、说完了；供应商侧工具（`hosted_tool`，核心 HT-1）的块另带
//! 结果（`outcome`）。

use serde_json::Value;

use super::{Block, Push};

/// 供应商侧工具的结果（`outcome`：`{"text", "failed"?, "cut"?}`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// 结果原文。
    pub text: String,
    /// 供应商说失败了。
    pub failed: bool,
}

impl Outcome {
    /// 读一块的 `outcome`；没有的是 `None`。
    pub fn read(outcome: &Value) -> Option<Self> {
        outcome.is_object().then(|| Self {
            text: outcome["text"].as_str().unwrap_or_default().to_string(),
            failed: outcome["failed"] == true,
        })
    }
}

/// 一块开始的种类：`start` 和 `name`，供应商侧工具另记谁跑的（事件 `by` 的端点）。
pub(super) fn block(start: &str, name: &Value, by: &Value) -> Block {
    let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
    match start {
        "text" => Block::Text,
        "reasoning" => Block::Reasoning,
        "hosted_tool" => Block::Hosted(text(name), text(&by["endpoint"])),
        _ => Block::ToolCall(text(name)),
    }
}

/// 读一条 `model.delta`。
pub(super) fn read(event: &Value, out: &mut Vec<Push>) {
    let body = &event["body"];
    let index = body["index"].as_u64().unwrap_or_default();
    if let Some(start) = body["start"].as_str() {
        if let Some(seen) = body["seen"].as_u64() {
            out.push(Push::Heard(seen));
        }
        let block = block(start, &body["name"], &event["by"]);
        out.push(Push::BlockStart { index, block });
    }
    if let Some(piece) = body["text"].as_str() {
        out.push(Push::Delta {
            index,
            text: piece.to_string(),
        });
    }
    if let Some(outcome) = Outcome::read(&body["outcome"]) {
        out.push(Push::HostedOutcome(index, outcome));
    }
    if body["end"] == true {
        out.push(Push::BlockEnd(index));
    }
}
