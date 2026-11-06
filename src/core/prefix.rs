//! 前缀要变（蓝图「前缀要变」，核心 sessions.md、PX-1）：握手声明 `caps.prefix_ask` 以后，会话空着时开新一轮（`session.send`、
//! `session.redo`、开轮的 `command.run`）不带 `prefix` 的，核心回 `prefix_changing` 先问人。这里记着最近一条开轮的请求（拼好的
//! 参数，附件已经传过），被拒时交界面问；人选了「直接发送」「压缩上下文并发送」照它原样加 `prefix` 再发一次。

use std::collections::HashMap;

use serde_json::{Value, json};

use super::awaiting::Awaiting;
use super::link::Link;
use super::rpc::Rpc;
use super::update::Update;

/// 最近一条开轮的请求：再发一次要的全在这里。
#[derive(Debug, Clone)]
pub struct Opened {
    /// 方法。
    pub method: &'static str,
    /// 拼好的参数（附件换成了传好的）。
    pub params: Value,
    /// 等回应的种类：再发一次照它办。
    pub kind: Awaiting,
}

/// 这种请求开不开轮、要不要记着。
pub(super) fn opens_turn(kind: &Awaiting) -> bool {
    matches!(
        kind,
        Awaiting::Send | Awaiting::Redo | Awaiting::CommandRun | Awaiting::Slash
    )
}

/// 开轮的请求记一份（发出去以前，`serve.rs`）：前缀要变、人选了以后照它再发。别的不记。
pub(super) fn record(
    kind: Option<&Awaiting>,
    method: &'static str,
    params: &Value,
) -> Option<Opened> {
    kind.filter(|k| opens_turn(k)).map(|k| Opened {
        method,
        params: params.clone(),
        kind: k.clone(),
    })
}

/// 回应是 `prefix_changing` 的：交界面的那一份（改了的文件、剩下几个；开轮命令的原话，取消时放回框里）。别的是 `None`。
pub(super) fn changing(error: &Value, kind: Option<&Awaiting>, link: &Link) -> Option<Update> {
    if error["data"]["reason"] != "prefix_changing" || !kind.is_some_and(opens_turn) {
        return None;
    }
    let data = &error["data"];
    let files = data["files"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| f.as_str().map(str::to_string))
        .collect();
    let command = link
        .opened
        .as_ref()
        .filter(|o| o.method == "command.run")
        .and_then(|o| o.params["text"].as_str())
        .map(str::to_string);
    Some(Update::PrefixChanging {
        files,
        more: data["more"].as_u64().unwrap_or(0),
        command,
    })
}

/// 人选了：照记着的那一条加 `prefix`（`keep`、`compact`）再发一次，回应照原来的种类等；没记着的（重连过、已经发过）不发。
/// 交回连接还在不在。
pub(super) async fn retry(
    rpc: &mut Rpc,
    link: &mut Link,
    choice: &'static str,
    awaiting: &mut HashMap<String, Awaiting>,
) -> bool {
    let Some(opened) = link.opened.take() else {
        return true;
    };
    let mut params = opened.params;
    params["prefix"] = json!(choice);
    let Ok(id) = rpc.send(opened.method, params).await else {
        return false;
    };
    awaiting.insert(id, opened.kind);
    true
}

#[cfg(test)]
mod tests;
