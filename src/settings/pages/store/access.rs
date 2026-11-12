//! 接入方式的标签（核心设计 48 第一节，2026-11-12 项目主人：包编号不再带 `miyu-` 和接入方式，谁出的、怎么接入交给界面上的
//! 标签说）：照核心已有的几格认，不加清单字段。和网页一样的几种、一样的先后，最多三个。
//!
//! 软件源一行（`package.search`）看 `kind`、`provides`；装着的一项（`package.list`）看 `kind`、`connection`、`mcp`、
//! `command`。一个都认不出的（出厂的内置包这类）没有标签。

use serde_json::Value;

/// 认得的几种，照画的先后；也是字表 `settings.more.store.access` 的键。
pub const KINDS: [&str; 9] = [
    "connection",
    "program",
    "mcp",
    "skills",
    "scripts",
    "theme",
    "mascot",
    "ui",
    "personas",
];

/// 最多画几个。
pub const MOST: usize = 3;

/// 一个包（`package.search` 的一行或 `package.list` 的一项）的标签，照 [`KINDS`] 的先后，最多 [`MOST`] 个。
pub fn of(p: &Value) -> Vec<&'static str> {
    let kind = p["kind"].as_str().unwrap_or_default();
    let carried = |key: &str| filled(&p["provides"][key]);
    KINDS
        .into_iter()
        .filter(|k| match *k {
            "connection" => filled(&p["connection"]) || kind == "connection",
            "program" => {
                kind == "process" || kind == "worker" || filled(&p["command"]) || carried("command")
            }
            "mcp" => filled(&p["mcp"]) || carried("mcp"),
            "theme" | "mascot" | "ui" => kind == *k,
            other => carried(other),
        })
        .take(MOST)
        .collect()
}

/// 有东西：空的数组、对象、字、`false`、`null` 都算没有。
fn filled(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

#[cfg(test)]
mod tests;
