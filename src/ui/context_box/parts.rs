//! 上一段，组成：一根条接用了多少，一块一行（子块缩进、暗色），右边 token 数和占已用的百分比；大块和大块之间空一行
//! （2026-10-23 项目主人）。数照 `used` 摊（蓝图「上下文 `/context`」第 3 条）。

use ratatui::text::{Line, Span};
use serde_json::Value;
use unicode_width::UnicodeWidthStr;

use super::{Texts, number, pad_left, pad_right};
use crate::meter;
use crate::theme;

/// `session.context` 的回应。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shape {
    /// 上下文窗口、压缩线（没交过限额的没有）。
    pub window: Option<u64>,
    pub line: Option<u64>,
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
        used: number(&got["used"]),
        estimated: got["basis"] == "estimated",
        parts: read(&got["parts"]),
    }
}

/// 一块一行要的：缩进几层、名字、多少。
fn flat(parts: &[Part], depth: usize, out: &mut Vec<(usize, String, u64)>) {
    for p in parts {
        out.push((depth, p.label.clone(), p.tokens));
        flat(&p.children, depth + 1, out);
    }
}

/// 排好的一行行。
pub(super) fn rows(
    shape: &Shape,
    texts: &Texts,
    marks: &crate::config::Bar,
    room: usize,
) -> Vec<Line<'static>> {
    let used = shape.used();
    let mut out = vec![head(shape, used, texts, marks, room), Line::default()];
    let mut list = Vec::new();
    flat(&shape.apportioned(), 0, &mut list);
    let name_width = list
        .iter()
        .map(|(depth, label, _)| depth * 2 + label.width())
        .max()
        .unwrap_or(0)
        + 2;
    let numbers: Vec<String> = list.iter().map(|(_, _, n)| meter::thousands(*n)).collect();
    let number_width = numbers.iter().map(|n| n.width()).max().unwrap_or(0);
    // 占已用的百分比：窗口大的（1M）占窗口的几乎都是 0%，看不出谁多（2026-10-23 真机自测）。
    let row = |depth: usize, label: &str, tokens: u64| {
        let name = pad_right(&format!("{}{label}", "  ".repeat(depth)), name_width);
        let share = format!("{}%", meter::percent_tenths(tokens, used));
        let style = if depth == 0 {
            ratatui::style::Style::default()
        } else {
            theme::dim()
        };
        Line::from(vec![
            Span::styled(name, style),
            Span::styled(pad_left(&meter::thousands(tokens), number_width), style),
            Span::styled(pad_left(&share, 8), theme::dim()),
        ])
    };
    for (n, (depth, label, tokens)) in list.iter().enumerate() {
        if *depth == 0 && n > 0 {
            out.push(Line::default());
        }
        out.push(row(*depth, label, *tokens));
    }
    out
}

/// 头一行：一根条（同侧边栏），接用了多少、压缩线；全是本地估的接「估算」。
fn head(
    shape: &Shape,
    used: u64,
    texts: &Texts,
    marks: &crate::config::Bar,
    room: usize,
) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let text = match shape.window {
        Some(window) => {
            let bar =
                crate::ui::sidebar::bar(used, window, shape.line, marks.width.min(room / 3), marks);
            spans.extend(bar.spans.into_iter().skip(1));
            spans.push(Span::raw("  "));
            texts
                .used
                .replace("{used}", &meter::short(used))
                .replace("{window}", &meter::short(window))
        }
        None => texts.used_only.replace("{used}", &meter::short(used)),
    };
    spans.push(Span::raw(text));
    let sep = crate::sheet::sep();
    if let Some(line) = shape.line {
        let line = texts.line.replace("{line}", &meter::short(line));
        spans.push(Span::styled(format!("{sep}{line}"), theme::dim()));
    }
    if shape.estimated {
        spans.push(Span::styled(
            format!("{sep}{}", texts.estimated),
            theme::dim(),
        ));
    }
    Line::from(spans)
}
