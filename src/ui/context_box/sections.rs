//! 三段里新加的那几行（蓝图「上下文 `/context`」第 7 条，反馈 1102 第 22 条，照网页 W-1053）：上下文那一段底下的压缩线、
//! 压缩次数、缓存断裂次数和卡住的原因；累计消耗的段头和四种 token；花费那一段。
//!
//! 只排，不碰界面：数由 `context_box.rs` 收好（[`Tail`]）。

use ratatui::text::{Line, Span};

use super::{Texts, pad_left, pad_right};
use crate::core::{Bill, Converted, Cost, KindMoney, Split, UsageRow};
use crate::meter;
use crate::theme;
use crate::transcript::Stuck;

/// 上下文那一段底下要写的：压缩了几次、缓存断了几次、卡住了没有（照正在看的会话）。压缩线照旧写在那根条后面（第 3 条）。
#[derive(Debug, Clone, Copy, Default)]
pub struct Tail {
    pub compactions: u64,
    pub breaks: u64,
    pub stuck: Option<Stuck>,
}

/// 名目在左、数在右的几行：名目暗色，数右对齐到同一列。
fn pairs(rows: &[(String, String)], room: usize) -> Vec<Line<'static>> {
    let name_width = rows.iter().map(|(n, _)| width(n)).max().unwrap_or(0) + 2;
    let value_width = rows.iter().map(|(_, v)| width(v)).max().unwrap_or(0);
    let value_width = value_width.min(room.saturating_sub(name_width));
    rows.iter()
        .map(|(name, value)| {
            Line::from(vec![
                Span::styled(pad_right(name, name_width), theme::dim()),
                Span::styled(pad_left(value, value_width), theme::dim()),
            ])
        })
        .collect()
}

fn width(text: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(text)
}

/// 上下文那一段底下：已压缩、缓存断裂；卡住的再一行红字说为什么。
pub(super) fn context_tail(tail: &Tail, texts: &Texts, room: usize) -> Vec<Line<'static>> {
    let times = |n: u64| texts.times.replace("{n}", &n.to_string());
    let rows = vec![
        (texts.compactions.clone(), times(tail.compactions)),
        (texts.breaks.clone(), times(tail.breaks)),
    ];
    let mut out = pairs(&rows, room);
    let why = match tail.stuck {
        Some(Stuck::Failed { .. }) => Some(&texts.stuck_failed),
        Some(Stuck::Over { .. }) => Some(&texts.stuck_over),
        None => None,
    };
    if let Some(why) = why {
        out.push(Line::styled(why.clone(), theme::error()));
    }
    out
}

/// 段头：左边正常色，右边暗色贴着右边。
fn head(left: String, right: Option<String>, room: usize) -> Line<'static> {
    let Some(right) = right else {
        return Line::raw(left);
    };
    let gap = room.saturating_sub(width(&left) + width(&right)).max(2);
    Line::from(vec![
        Span::raw(left),
        Span::raw(" ".repeat(gap)),
        Span::styled(right, theme::dim()),
    ])
}

/// 按用途那几行（一次都没有的不算）加起来的四种 token。
fn split_of(list: &[UsageRow]) -> Split {
    let mut total = Split::default();
    for r in list.iter().filter(|r| r.requests > 0) {
        total.uncached += r.split.uncached;
        total.cache_read += r.split.cache_read;
        total.cache_write += r.split.cache_write;
        total.output += r.split.output;
    }
    total
}

/// 累计消耗的段头和四种 token 一种一行（缓存读、缓存写、输出、输入（未命中）：数、占累计的百分比）；一次都没有的是空的。
pub(super) fn spent(list: &[UsageRow], texts: &Texts, room: usize) -> Vec<Line<'static>> {
    let split = split_of(list);
    let all = split.uncached + split.cache_read + split.cache_write + split.output;
    if all == 0 {
        return Vec::new();
    }
    let input = split.uncached + split.cache_read + split.cache_write;
    let hit = texts
        .hit
        .replace("{percent}", &meter::hit_rate(split.cache_read, input));
    let title = texts.spent.replace("{tokens}", &meter::short(all));
    let mut out = vec![head(title, Some(hit), room)];
    let counts = [
        split.cache_read,
        split.cache_write,
        split.output,
        split.uncached,
    ];
    let rows: Vec<(String, String)> = texts
        .kinds
        .iter()
        .zip(counts)
        .map(|(name, n)| {
            let share = format!("{}%", meter::percent_tenths(n, all));
            (
                name.clone(),
                format!("{}  {}", meter::short(n), pad_left(&share, 6)),
            )
        })
        .collect();
    out.extend(pairs(&rows, room));
    out
}

/// 花费那一段：段头「花费」右边总数（照按用途那几行的 `converted` 加起来，`money` 照界面的币种写）；算不出钱的写暗字
/// 「没有价格」。下面按种类一种一行（`converted_parts` 加起来，核心 UP-1）：有一行没有、某一种不知道的，那一种不写。
/// 一次都没有的是空的。
pub(super) fn cost(
    list: &[UsageRow],
    texts: &Texts,
    money: super::Money,
    room: usize,
) -> Vec<Line<'static>> {
    let used: Vec<&UsageRow> = list.iter().filter(|r| r.requests > 0).collect();
    if used.is_empty() {
        return Vec::new();
    }
    let mut bill = Bill::default();
    used.iter().for_each(|r| bill.merge(&r.bill));
    let Some(total) = money(&bill) else {
        return vec![head(texts.cost.clone(), Some(texts.no_price.clone()), room)];
    };
    let mut out = vec![head(texts.cost.clone(), Some(total), room)];
    let Some(converted) = bill.converted.as_ref() else {
        return out;
    };
    let Some(kinds) = KindMoney::sum(used.iter().map(|r| r.kinds.as_ref())) else {
        return out;
    };
    let sum: f64 = kinds.all().iter().flatten().sum();
    let one = |amount: f64| {
        let only = Bill {
            converted: Some(Converted {
                total: Cost {
                    amount,
                    currency: converted.total.currency.clone(),
                },
                rest: Vec::new(),
            }),
            ..Bill::default()
        };
        money(&only).unwrap_or_default()
    };
    let rows: Vec<(String, String)> = texts
        .kinds
        .iter()
        .zip(kinds.all())
        .filter_map(|(name, amount)| {
            let amount = amount?;
            // 照 token 那几行的写法（`meter::percent_tenths`）：钱换成十亿分之一的整数比。
            let scale = |v: f64| (v * 1e9).round() as u64;
            let share = format!("{}%", meter::percent_tenths(scale(amount), scale(sum)));
            Some((
                name.clone(),
                format!("{}  {}", one(amount), pad_left(&share, 6)),
            ))
        })
        .collect();
    out.extend(pairs(&rows, room));
    out
}
