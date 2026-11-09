//! 上一段，组成：段头右边写用了多少，一块一行一根小条（子块缩进、暗色），右边 token 数和占已用的百分比（发版前 check 32
//! 改成小条）。数照 `used` 摊（蓝图「上下文 `/context`」第 3 条）。

use ratatui::text::Line;
use serde_json::Value;

use super::{Texts, bars, number};
use crate::meter;

/// `session.context` 的回应。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shape {
    /// 上下文窗口、压缩线（没交过限额的没有）。
    pub window: Option<u64>,
    pub line: Option<u64>,
    /// 这个会话设的上下文上限（核心 CTX-1，设计 68）：`window` 已经取了小的；没设的没有。
    pub context: Option<u64>,
    /// 这时用了多少（口径同底栏）；算不了的没有。
    pub used: Option<u64>,
    /// 全是本地估的（`basis: estimated`）。
    pub estimated: bool,
    /// 一块块。
    pub parts: Vec<Part>,
}

/// 一块：名字（照连接的语言）、多少、下面的小块。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Part {
    pub label: String,
    pub tokens: u64,
    pub children: Vec<Part>,
}

impl Shape {
    /// 用了多少：核心给的，没给的是各块之和。
    pub fn used(&self) -> u64 {
        self.used.unwrap_or_else(|| self.sum())
    }

    fn sum(&self) -> u64 {
        self.parts.iter().map(|p| p.tokens).sum()
    }

    /// 各块照 `used` 摊：加起来和 `used` 不一样的照 `used ÷ 各块之和` 一起缩放（核心 CX-1 下以后自己摊好，比例是 1）。
    pub fn apportioned(&self) -> Vec<Part> {
        let sum = self.sum();
        let Some(used) = self.used.filter(|u| *u != sum && sum > 0) else {
            return self.parts.clone();
        };
        let ratio = used as f64 / sum as f64;
        fn scale(parts: &[Part], ratio: f64) -> Vec<Part> {
            parts
                .iter()
                .map(|p| Part {
                    label: p.label.clone(),
                    tokens: (p.tokens as f64 * ratio).round() as u64,
                    children: scale(&p.children, ratio),
                })
                .collect()
        }
        scale(&self.parts, ratio)
    }
}

/// 读 `session.context` 的回应；读不懂的块不要。
pub fn read_parts(got: &Value) -> Shape {
    fn read(list: &Value) -> Vec<Part> {
        list.as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| {
                Some(Part {
                    label: p["label"]
                        .as_str()
                        .or_else(|| p["part"].as_str())?
                        .to_string(),
                    tokens: number(&p["tokens"])?,
                    children: read(&p["children"]),
                })
            })
            .collect()
    }
    Shape {
        window: number(&got["window"]).filter(|w| *w > 0),
        line: number(&got["line"]),
        context: number(&got["context"]),
        used: number(&got["used"]),
        estimated: got["basis"] == "estimated",
        parts: read(&got["parts"]),
    }
}

/// 一块一行要的：缩进几层、名字、多少。
fn flat(parts: &[Part], depth: usize, out: &mut Vec<bars::Row>) {
    for p in parts {
        out.push((depth, p.label.clone(), p.tokens));
        flat(&p.children, depth + 1, out);
    }
}

/// 排好的一行行：段头「上下文」右边暗字写用了多少（发版前 check 32：不再画那根大条），下面一块一行一根小条（`bars.rs`），
/// 大块之间不再空行。
pub(super) fn rows(shape: &Shape, texts: &Texts, room: usize) -> Vec<Line<'static>> {
    let used = shape.used();
    let mut list = Vec::new();
    flat(&shape.apportioned(), 0, &mut list);
    let laid = bars::rows(&list, used, room);
    let mut out = vec![bars::head(
        &texts.context,
        &summary(shape, used, texts),
        laid.width,
    )];
    out.extend(laid.lines);
    out
}

/// 段头右边：用了多少 / 窗口（没有窗口的只写用了多少），接压缩线；全是本地估的接「估算」。
fn summary(shape: &Shape, used: u64, texts: &Texts) -> String {
    let sep = crate::sheet::sep();
    let mut text = match shape.window {
        Some(window) => texts
            .used
            .replace("{used}", &meter::short(used))
            .replace("{window}", &meter::short(window)),
        None => texts.used_only.replace("{used}", &meter::short(used)),
    };
    if let Some(line) = shape.line {
        text.push_str(&format!(
            "{sep}{}",
            texts.line.replace("{line}", &meter::short(line))
        ));
    }
    // 设了会话上限的：窗口是被它压小的，接着写（设计 68：人知道是哪一格在起作用）。
    if let Some(cap) = shape.context {
        let cap = texts.cap.replace("{n}", &meter::short(cap));
        text.push_str(&format!("{sep}{cap}"));
    }
    if shape.estimated {
        text.push_str(&format!("{sep}{}", texts.estimated));
    }
    text
}
