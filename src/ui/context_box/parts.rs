//! 上一段，现在的上下文：段头右边写用了多少，一根进度条、离压缩线多远，各大块一行一根小条（多的在上），右边 token 数和
//! 占已用的百分比；小块默认收着。数照 `used` 摊（蓝图「上下文 `/context`」第 3 条）。

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

/// 一块一行要的：缩进几层、名字、多少。大块照多少从大到小排（一样多的照核心给的先后）；`open` 才接着写它下面的小块。
fn flat(parts: &[Part], open: bool) -> Vec<bars::Row> {
    fn under(parts: &[Part], depth: usize, out: &mut Vec<bars::Row>) {
        for p in parts {
            out.push((depth, p.label.clone(), p.tokens));
            under(&p.children, depth + 1, out);
        }
    }
    let mut sorted: Vec<&Part> = parts.iter().collect();
    sorted.sort_by_key(|p| std::cmp::Reverse(p.tokens));
    let mut out = Vec::new();
    for p in sorted {
        out.push((0, p.label.clone(), p.tokens));
        if open {
            under(&p.children, 1, &mut out);
        }
    }
    out
}

/// 排好的一行行（2026-11-11 项目主人挑的样板）：段头「上下文」右边写用了多少、占窗口几成；下面一根进度条（压缩线那一格
/// 标黄，同侧边栏），再一行暗字离压缩线多远；卡住的红字说为什么（`why`）；空一行，各大块一行一根小条（`bars.rs`），小块
/// 展开了（`open`）才写、不画条。
pub(super) fn rows(
    shape: &Shape,
    (texts, marks): (&Texts, &crate::config::Bar),
    (room, open): (usize, bool),
    why: Option<Line<'static>>,
) -> Vec<Line<'static>> {
    let used = shape.used();
    let laid = bars::rows(&flat(&shape.apportioned(), open), used, room);
    let mut out = vec![bars::head(
        &texts.context,
        &summary(shape, used, texts),
        laid.width,
    )];
    if let Some(window) = shape.window {
        let meter = crate::ui::sidebar::bar(used, window, shape.line, laid.width.min(room), marks);
        out.push(Line::from(
            meter.spans.into_iter().skip(1).collect::<Vec<_>>(),
        ));
    }
    if let Some(text) = margin(shape, used, texts) {
        out.push(Line::styled(text, crate::theme::dim()));
    }
    out.extend(why);
    out.push(Line::default());
    out.extend(laid.lines);
    out
}

/// 段头右边：用了多少 / 窗口、占窗口几成（没有窗口的只写用了多少）；全是本地估的接「估算」。
fn summary(shape: &Shape, used: u64, texts: &Texts) -> String {
    let mut text = match shape.window {
        Some(window) => format!(
            "{}  {}%",
            texts
                .used
                .replace("{used}", &meter::short(used))
                .replace("{window}", &meter::short(window)),
            meter::percent_tenths(used, window)
        ),
        None => texts.used_only.replace("{used}", &meter::short(used)),
    };
    if shape.estimated {
        text.push_str(&format!("{}{}", crate::sheet::sep(), texts.estimated));
    }
    text
}

/// 进度条下面那一行：离压缩线还差多少（过了的写过了多少）；设了会话上限的接着写（设计 68：人知道窗口是被哪一格压小的）。
/// 两样都没有的不写。
fn margin(shape: &Shape, used: u64, texts: &Texts) -> Option<String> {
    let line = shape.line.map(|line| {
        if used <= line {
            texts.to_line.replace("{n}", &meter::short(line - used))
        } else {
            texts.past_line.replace("{n}", &meter::short(used - line))
        }
    });
    let cap = shape
        .context
        .map(|cap| texts.cap.replace("{n}", &meter::short(cap)));
    let said: Vec<String> = line.into_iter().chain(cap).collect();
    (!said.is_empty()).then(|| said.join(&crate::sheet::sep()))
}
