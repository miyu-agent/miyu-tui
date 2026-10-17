//! 工具结果里给人看的结构化结果（`detail`，核心 D-6）。

use serde_json::{Value, json};

use super::read_mine;
use crate::core::push::Push;

#[test]
fn a_tool_result_carries_its_detail_for_people() {
    // 核心 D-6：给人看的结构化结果（翻记忆、查知识库、读文档），不发给模型；老日志、出错的没有这一格。
    let body = |detail: Value| {
        json!({"kind": "tool.result", "by": {"kind": "tool"},
            "body": {"call_id": "c1", "status": "ok", "blocks": [{"type": "text", "text": "Nothing found."}], "detail": detail}})
    };
    let detail = |event: &Value| match &read_mine(event)[0] {
        Push::ToolResult { detail, .. } => detail.clone(),
        other => panic!("{other:?}"),
    };
    let hits = json!({"hits": [{"kb": "notes", "path": "a.md", "snippet": "…"}]});
    assert_eq!(detail(&body(hits.clone())), Some(hits));
    assert_eq!(detail(&body(Value::Null)), None, "没有这一格");
    assert_eq!(detail(&body(json!("text"))), None, "不是对象的不要");
}
