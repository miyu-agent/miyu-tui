//! 人打的 `/dream` 跟着看整理（核心 R-21，`docs/reference/memory.md`「`memory.dream`」；蓝图 `tui.md`「整理记忆」）：订阅这个
//! 会话那一间的 `memory` 流，整理完退订。日志里开始合、合完、没合成的三行和不落盘的 `memory.dreaming` 读成
//! [`Change`] 交给界面；每开始合一批，照编号取一次正文（`memory.list` 带作废的），浮过去、删掉的原文照它写。

use std::collections::HashMap;
use std::io;

use serde_json::{Value, json};

use super::Update;
use super::rpc::Rpc;
use crate::dream::{Change, Retire};

/// 一间至多列这么多条（核心的上限）。
const LIST_LIMIT: u64 = 500;

/// 订阅 `session` 那一间的记忆流；`None` 是退订。交回订阅的请求编号（回应不用看）。
pub(super) async fn follow(rpc: &mut Rpc, session: Option<&str>) -> io::Result<Option<String>> {
    match session {
        Some(session) => {
            let params = json!({"stream": "memory", "session": session});
            rpc.send("subscribe", params).await.map(Some)
        }
        None => {
            rpc.send("unsubscribe", json!({"stream": "memory"})).await?;
            Ok(None)
        }
    }
}

/// 取这一间的正文（带作废的：删掉的原文也要）。
pub(super) async fn list(rpc: &mut Rpc, session: &str) -> io::Result<String> {
    let params = json!({"session": session, "forgotten": true, "limit": LIST_LIMIT});
    rpc.send("memory.list", params).await
}

/// `memory.list` 的回应：编号到正文。
pub(super) fn texts(session: String, result: &Value) -> Update {
    let texts: HashMap<String, String> = result["memories"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| {
            Some((
                m["id"].as_str()?.to_string(),
                m["text"].as_str()?.to_string(),
            ))
        })
        .collect();
    Update::Dream {
        session,
        change: Change::Texts(texts),
    }
}

/// 记忆流推来的一条：和整理有关的读成 [`Update::Dream`]；不是记忆流的是 `None`，是记忆流、和整理无关的交回
/// `Some(None)`（别再往下当会话的事件走：它也带 `session`）。
pub(super) fn pushed(message: &Value) -> Option<Option<Update>> {
    let params = &message["params"];
    let change = match message["method"].as_str()? {
        "memory.event" => event(&params["event"]),
        "memory.dreaming" => dreaming(params),
        _ => return None,
    };
    let session = params["session"].as_str();
    Some(change.zip(session).map(|(change, session)| Update::Dream {
        session: session.to_string(),
        change,
    }))
}

/// 日志里的一行：开始合、合完、没合成。
fn event(event: &Value) -> Option<Change> {
    let body = &event["body"];
    let upto = body["upto"].as_u64()?;
    match event["kind"].as_str()? {
        "ext.memory.merging" => Some(Change::Merging {
            upto,
            fresh: body["fresh"].as_u64().unwrap_or(0),
            left: body["left"].as_u64().unwrap_or(0),
            given: body["given"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|id| id.as_str().map(str::to_string))
                .collect(),
        }),
        "ext.memory.merged" => Some(Change::Merged { upto }),
        "ext.memory.merge_failed" => Some(Change::Failed { upto }),
        _ => None,
    }
}

/// 她刚写完的一条（`memory.dreaming`）。
fn dreaming(params: &Value) -> Option<Change> {
    let text = |key: &str| params[key].as_str().map(str::to_string);
    match params["kind"].as_str()? {
        "revised" => Some(Change::Revised {
            text: text("text")?,
        }),
        "retired" => Some(Change::Retired {
            id: text("id")?,
            why: Retire::read(params["retire"].as_str().unwrap_or_default()),
        }),
        "summary" => Some(Change::Summary {
            chars: params["chars"].as_u64().unwrap_or(0),
            done: params["done"].as_bool().unwrap_or(false),
        }),
        _ => None,
    }
}

/// 开始合一批：要再取一次正文（新记的可能是刚抽出来的）。
pub(super) fn wants_texts(update: &Update) -> Option<&str> {
    match update {
        Update::Dream {
            session,
            change: Change::Merging { .. },
        } => Some(session),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
