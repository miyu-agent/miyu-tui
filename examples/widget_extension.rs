//! 测试用的扩展（`tests/widgets.rs`）：伪终端测试把它放进包的 `bin/`，让核心拉起来，往界面上放块。`cargo test` 会编
//! examples，发行不带它。只用标准库和 `serde_json`。
//!
//! 参数：
//!
//! - `content:<路径>`：每一块推什么，一个 JSON 对象：块的编号 → `{"title"?, "content"}`；核心说有人看（`widget.watch`）时
//!   照它推 `widget.set`（每次现读，测试可以中途改）；
//! - `record:<路径>`：收到的 `widget.watch`、`method.call` 一行一条记下；
//! - `sticker:<blob>`：登记一件工具 `sticker`，调到时答「Sent.」，`sent` 里是这个 blob 的一张表情包（核心 X-19，
//!   `tests/sent.rs`）。
//!
//! 登记一个方法 `refresh`：照 `params.widget` 把那一块换成一段字 `refreshed`，再答。

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

fn main() {
    let mut content = None;
    let mut record = None;
    let mut sticker = None;
    for arg in std::env::args().skip(1) {
        if let Some(path) = arg.strip_prefix("content:") {
            content = Some(path.to_string());
        } else if let Some(path) = arg.strip_prefix("record:") {
            record = Some(path.to_string());
        } else if let Some(blob) = arg.strip_prefix("sticker:") {
            sticker = Some(blob.to_string());
        }
    }
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    send(&json!({"jsonrpc": "2.0", "id": "h", "method": "hello",
        "params": {"protocol": [1, 1], "head": {"kind": "test-extension", "version": "0"}}}));
    // 等握手的回应再登记方法。
    for line in lines.by_ref() {
        let Ok(line) = line else { return };
        if serde_json::from_str::<Value>(&line).is_ok_and(|m| m["id"] == "h") {
            break;
        }
    }
    send(
        &json!({"jsonrpc": "2.0", "id": "m", "method": "package.methods",
        "params": {"methods": [{"name": "refresh"}]}}),
    );
    if sticker.is_some() {
        send(
            &json!({"jsonrpc": "2.0", "id": "p", "method": "provide", "params": {"tools": [
            {"name": "sticker", "description": "Send a sticker.", "input_schema": {"type": "object"},
             "access": "read", "venues": ["local"]}]}}),
        );
    }
    // 每一块最近一次有人看时的会话：`refresh` 照它推。
    let mut sessions: HashMap<String, Option<String>> = HashMap::new();
    let mut n = 0;
    while let Some(Ok(line)) = lines.next() {
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let params = &message["params"];
        match message["method"].as_str() {
            Some("widget.watch") => {
                keep(record.as_deref(), &line);
                let Some(widget) = params["widget"].as_str() else {
                    continue;
                };
                let session = params["session"].as_str().map(str::to_string);
                sessions.insert(widget.to_string(), session.clone());
                let wanted = content
                    .as_deref()
                    .and_then(|p| std::fs::read_to_string(p).ok())
                    .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                    .map(|all| all[widget].clone())
                    .unwrap_or(Value::Null);
                if wanted.is_null() {
                    continue;
                }
                n += 1;
                set(n, widget, session.as_deref(), &wanted);
            }
            Some("method.call") => {
                keep(record.as_deref(), &line);
                let widget = params["params"]["widget"].as_str().unwrap_or_default();
                if params["method"] == "refresh" && sessions.contains_key(widget) {
                    n += 1;
                    let session = sessions.get(widget).cloned().flatten();
                    let refreshed = json!({"content": [{"type": "text", "text": "refreshed"}]});
                    set(n, widget, session.as_deref(), &refreshed);
                }
                send(
                    &json!({"jsonrpc": "2.0", "id": message["id"], "result": {"answered": params["method"]}}),
                );
            }
            Some("tool.call") if params["tool"] == "sticker" => {
                let sent = json!([{"blob": sticker, "media_type": "image/png", "width": 2, "height": 2,
                    "alt": "招手的猫", "sticker": true}]);
                send(&json!({"jsonrpc": "2.0", "id": message["id"],
                    "result": {"blocks": [{"type": "text", "text": "Sent."}], "sent": sent}}));
            }
            _ => {}
        }
    }
}

/// 记下收到的一行。
fn keep(record: Option<&str>, line: &str) {
    if let Some(path) = record
        && let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path)
    {
        drop(writeln!(file, "{line}"));
    }
}

/// 推一块：`wanted` 是 `{"title"?, "content"}`。
fn set(n: u64, widget: &str, session: Option<&str>, wanted: &Value) {
    let mut params = json!({"widget": widget, "content": wanted["content"]});
    if let Some(session) = session {
        params["session"] = json!(session);
    }
    if let Some(title) = wanted.get("title") {
        params["title"] = title.clone();
    }
    send(
        &json!({"jsonrpc": "2.0", "id": format!("w-{n}"), "method": "widget.set", "params": params}),
    );
}

fn send(message: &Value) {
    let mut out = io::stdout().lock();
    drop(writeln!(out, "{message}"));
    drop(out.flush());
}
