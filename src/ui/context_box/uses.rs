//! 按用途（蓝图「上下文 `/context`」第 4 条）：一行一种用途，照 token 多少排；列是次数、token（四种加起来）、花费，不止一行的
//! 最后一行合计（发版前 check 32，2026-11-09 项目主人挑的样板：原来的六列表拆成三列，四种 token 挪到上面那一段的条里）。
//! 第一行暗字「按用途」兼列名。窄得放不下时去掉花费。

use ratatui::text::{Line, Span};
use serde_json::Value;
use unicode_width::UnicodeWidthStr;

use super::{Texts, pad_left, pad_right};
use crate::core::{Bill, UsageRow};
use crate::meter;
use crate::theme;

/// 读 `usage.query` 按 `use` 分的回应（同 `/usage` 的一行行）。
pub fn read_uses(got: &Value) -> Vec<UsageRow> {
    crate::core::usage_rows(got)
}

/// 没价格的写它。
const NO_PRICE: &str = "—";

/// 一行的几格：名字、次数、token、花费。
struct Cells {
    name: String,
    requests: u64,
    tokens: u64,
    cost: String,
}

/// 一行用掉的四种 token 加起来。
fn tokens_of(r: &UsageRow) -> u64 {
    r.split.uncached + r.split.cache_read + r.split.cache_write + r.split.output
}

/// 排好的一行行；一次都没有的是空的。
pub(super) fn rows(
    list: &[UsageRow],
    texts: &Texts,
    money: &dyn Fn(&Bill) -> Option<String>,
    room: usize,
) -> Vec<Line<'static>> {
    let mut sorted: Vec<&UsageRow> = list.iter().filter(|r| r.requests > 0).collect();
    if sorted.is_empty() {
        return Vec::new();
    }
    sorted.sort_by_key(|r| std::cmp::Reverse(r.tokens));
    let cost = |bill: &Bill| money(bill).unwrap_or_else(|| NO_PRICE.to_string());
    let mut table: Vec<Cells> = sorted
        .iter()
        .map(|r| {
            let id = r.used_for.clone().unwrap_or_default();
            Cells {
                name: texts.uses.get(&id).cloned().unwrap_or(id),
                requests: r.requests,
                tokens: tokens_of(r),
                cost: cost(&r.bill),
            }
        })
        .collect();
    if sorted.len() > 1 {
        let mut bill = Bill::default();
        sorted.iter().for_each(|r| bill.merge(&r.bill));
        table.push(Cells {
            name: texts.sum.clone(),
            requests: sorted.iter().map(|r| r.requests).sum(),
            tokens: sorted.iter().map(|r| tokens_of(r)).sum(),
            cost: cost(&bill),
        });
    }
    layout(&table, texts, room)
}

/// 列宽照最宽的那格（列名也算）；放不下的去掉花费。全是暗字（2026-10-23 项目主人：内容暗色）。
fn layout(table: &[Cells], texts: &Texts, room: usize) -> Vec<Line<'static>> {
    let c = &texts.columns;
    let name_width = table
        .iter()
        .map(|r| r.name.width())
        .chain([texts.totals.width()])
        .max()
        .unwrap_or(0)
        + 2;
    let wide = |head: &str, cells: &mut dyn Iterator<Item = String>| {
        cells
            .map(|s| s.width())
            .chain([head.width()])
            .max()
            .unwrap_or(0)
            + 2
    };
    let requests_width = wide(
        &c.requests,
        &mut table.iter().map(|r| r.requests.to_string()),
    );
    let tokens_width = wide(&c.tokens, &mut table.iter().map(|r| meter::short(r.tokens)));
    let cost_width = wide(&c.cost, &mut table.iter().map(|r| r.cost.clone()));
    let show_cost = name_width + requests_width + tokens_width + cost_width <= room;
    let line = |name: &str, requests: &str, tokens: &str, cost: &str| {
        let mut spans = vec![
            Span::styled(pad_right(name, name_width), theme::dim()),
            Span::styled(pad_left(requests, requests_width), theme::dim()),
            Span::styled(pad_left(tokens, tokens_width), theme::dim()),
        ];
        if show_cost {
            spans.push(Span::styled(pad_left(cost, cost_width), theme::dim()));
        }
        Line::from(spans)
    };
    let mut out = vec![line(&texts.totals, &c.requests, &c.tokens, &c.cost)];
    for r in table {
        let tokens = meter::short(r.tokens);
        out.push(line(&r.name, &r.requests.to_string(), &tokens, &r.cost));
    }
    out
}
