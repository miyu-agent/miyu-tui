//! 下一段，本会话累计（蓝图「上下文 `/context`」第 4 条）：一行一种用途，照 token 多少排；列是次数、输入（没命中缓存的）、
//! 缓存读、缓存写、输出、花费，最后一行合计。窄得放不下时先去掉缓存写、再去掉缓存读。

use ratatui::text::{Line, Span};
use serde_json::Value;
use unicode_width::UnicodeWidthStr;

use super::{Texts, pad_left, pad_right};
use crate::core::{Bill, Split, UsageRow};
use crate::meter;
use crate::theme;

/// 读 `usage.query` 按 `use` 分的回应（同 `/usage` 的一行行）。
pub fn read_uses(got: &Value) -> Vec<UsageRow> {
    crate::core::usage_rows(got)
}

/// 没价格的写它。
const NO_PRICE: &str = "—";

/// 一行的几格：名字，次数、四项用量，花费。
struct Cells {
    name: String,
    numbers: [u64; 5],
    cost: String,
}

fn cells(name: String, requests: u64, split: &Split, cost: String) -> Cells {
    Cells {
        name,
        numbers: [
            requests,
            split.uncached,
            split.cache_read,
            split.cache_write,
            split.output,
        ],
        cost,
    }
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
            let name = texts.uses.get(&id).cloned().unwrap_or(id);
            cells(name, r.requests, &r.split, cost(&r.bill))
        })
        .collect();
    let mut total = Split::default();
    let mut bill = Bill::default();
    for r in &sorted {
        total.uncached += r.split.uncached;
        total.cache_read += r.split.cache_read;
        total.cache_write += r.split.cache_write;
        total.output += r.split.output;
        bill.merge(&r.bill);
    }
    let requests = sorted.iter().map(|r| r.requests).sum();
    table.push(cells(texts.sum.clone(), requests, &total, cost(&bill)));
    layout(&table, texts, room)
}

/// 列宽照最宽的那格（列名也算），放不下的先去掉缓存写、再去掉缓存读。
fn layout(table: &[Cells], texts: &Texts, room: usize) -> Vec<Line<'static>> {
    let c = &texts.columns;
    let heads = [
        &c.requests,
        &c.uncached,
        &c.cache_read,
        &c.cache_write,
        &c.output,
    ];
    let name_width = table.iter().map(|r| r.name.width()).max().unwrap_or(0) + 2;
    let width_of = |i: usize| {
        table
            .iter()
            .map(|r| meter::short(r.numbers[i]).width())
            .chain([heads[i].width()])
            .max()
            .unwrap_or(0)
            + 2
    };
    let cost_width = table
        .iter()
        .map(|r| r.cost.width())
        .chain([c.cost.width()])
        .max()
        .unwrap_or(0)
        + 2;
    let mut shown: Vec<usize> = (0..5).collect();
    for drop in [3, 2] {
        let need: usize =
            name_width + shown.iter().map(|i| width_of(*i)).sum::<usize>() + cost_width;
        if need <= room {
            break;
        }
        shown.retain(|i| *i != drop);
    }
    // 标题正常色，下面的表都是暗色，合计也不另亮（2026-10-23 项目主人：小标题正常色、内容暗色）。
    let mut out = vec![Line::raw(texts.totals.clone())];
    let mut head = vec![Span::raw(pad_right("", name_width))];
    for i in &shown {
        head.push(Span::styled(
            pad_left(heads[*i], width_of(*i)),
            theme::dim(),
        ));
    }
    head.push(Span::styled(pad_left(&c.cost, cost_width), theme::dim()));
    out.push(Line::from(head));
    for r in table {
        let style = theme::dim();
        let mut spans = vec![Span::styled(pad_right(&r.name, name_width), style)];
        for i in &shown {
            spans.push(Span::styled(
                pad_left(&meter::short(r.numbers[*i]), width_of(*i)),
                style,
            ));
        }
        spans.push(Span::styled(pad_left(&r.cost, cost_width), style));
        out.push(Line::from(spans));
    }
    out
}
