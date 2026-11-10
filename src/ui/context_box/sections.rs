//! 两段里零碎的几行（蓝图「上下文 `/context`」第 7 条）：压缩次数、缓存断裂次数、卡住的原因；累计消耗的段头（一共多少
//! token、多少钱）和它下面那一行。2026-11-11 项目主人挑的样板起四种 token 的条不再画，只留一句缓存命中。
//!
//! 只排，不碰界面：数由 `context_box.rs` 收好（[`Tail`]）。

use ratatui::text::Line;

use super::{Texts, bars};
use crate::core::{Bill, Split, UsageRow};
use crate::meter;
use crate::theme;
use crate::transcript::Stuck;

/// 压缩了几次、缓存断了几次、卡住了没有（照正在看的会话）。
#[derive(Debug, Clone, Copy, Default)]
pub struct Tail {
    pub compactions: u64,
    pub breaks: u64,
    pub stuck: Option<Stuck>,
}

/// 「已压缩 N 次 · 缓存断裂 N 次」。
pub(super) fn counts(tail: &Tail, texts: &Texts) -> String {
    let times = |n: u64| texts.times.replace("{n}", &n.to_string());
    format!(
        "{} {}{}{} {}",
        texts.compactions,
        times(tail.compactions),
        crate::sheet::sep(),
        texts.breaks,
        times(tail.breaks)
    )
}

/// 卡住的一行红字说为什么；没卡住的没有。
pub(super) fn stuck(tail: &Tail, texts: &Texts) -> Option<Line<'static>> {
    let why = match tail.stuck {
        Some(Stuck::Failed { .. }) => &texts.stuck_failed,
        Some(Stuck::Over { .. }) => &texts.stuck_over,
        None => return None,
    };
    Some(Line::styled(why.clone(), theme::error()))
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

/// 累计消耗的段头：右边这个会话连子代理一共用了多少 token、花了多少钱（算不出钱的不写），收尾和下面那张表一列；下面
/// 一行暗字「缓存命中 N% · 已压缩 N 次 · 缓存断裂 N 次」。一次都没有的是空的。
pub(super) fn spent(
    list: &[UsageRow],
    texts: &Texts,
    money: &dyn Fn(&Bill) -> Option<String>,
    (counts, table): (&str, &[Line<'static>]),
) -> Vec<Line<'static>> {
    let split = split_of(list);
    let all = split.uncached + split.cache_read + split.cache_write + split.output;
    if all == 0 {
        return Vec::new();
    }
    let input = split.uncached + split.cache_read + split.cache_write;
    let mut bill = Bill::default();
    list.iter()
        .filter(|r| r.requests > 0)
        .for_each(|r| bill.merge(&r.bill));
    let mut right = meter::short(all);
    if let Some(cost) = money(&bill) {
        right.push_str(&format!("  {cost}"));
    }
    let width = table.iter().map(Line::width).max().unwrap_or(0);
    let hit = texts
        .hit
        .replace("{percent}", &meter::hit_rate(split.cache_read, input));
    vec![
        bars::head(&texts.spent, &right, width),
        Line::styled(
            format!("{hit}{}{counts}", crate::sheet::sep()),
            theme::dim(),
        ),
    ]
}
