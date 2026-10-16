//! 一件工具的结果（`tool.result`）：状态、文字、给人看的那一句、调用那一刻的显示名，结果里的图、她发出去的图各另交一条。

use std::collections::BTreeMap;

use serde_json::Value;

use super::{Push, blocks, jobs, sent};
use crate::core::kinds::ToolStatus;

/// 读一条 `tool.result`，照先后放进 `out`。
pub(super) fn read(body: &Value, out: &mut Vec<Push>) {
    let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
    jobs::effects(body, out);
    out.push(Push::ToolResult {
        call_id: text(&body["call_id"]),
        status: ToolStatus::parse(&text(&body["status"])),
        text: blocks(body)
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        said: serde_json::from_value(body["human"].clone()).ok(),
        display: display(&body["display_name"]),
        sent: body["sent"].as_array().map_or(0, Vec::len),
    });
    // 结果里的图（读了图片文件的 `read`）另交一条：时间线把图接在这一步下面（蓝图「时间线」第 8 条）。
    let blobs: Vec<String> = blocks(body)
        .filter(|b| b["type"] == "image")
        .filter_map(|b| b["blob"].as_str().map(str::to_string))
        .collect();
    if !blobs.is_empty() {
        out.push(Push::ToolImages {
            call_id: text(&body["call_id"]),
            blobs,
        });
    }
    // 她发出去的图（核心 X-19）另交一条：正文把它画在这一步后面，和回答一样收起前面那一段（蓝图「她发的图」）。
    let images = sent::read(&body["sent"]);
    if !images.is_empty() {
        out.push(Push::Media(images));
    }
}

/// 调用那一刻的显示名（核心 9-8 六补）：语言到名字，空的、不是字的不要；没有这一格的（核心自带的工具、内核替它写的
/// 结果、老日志）是空的。
fn display(names: &Value) -> BTreeMap<String, String> {
    names
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(lang, name)| {
            let name = name.as_str()?.trim();
            (!name.is_empty()).then(|| (lang.clone(), name.to_string()))
        })
        .collect()
}
