//! 三段里新加的那几行（蓝图「上下文 `/context`」第 7 条，反馈 1102 第 22 条，照网页 W-1053）：上下文那一段底下的压缩次数、
//! 缓存断裂次数和卡住的原因；累计消耗的段头和四种 token。发版前 check 32（2026-11-09 项目主人挑的「每行一根小条」）起四种
//! token 一种一根条，单独的「花费」那一段（按种类分的钱）不再写，钱只在按用途那张表里。
//!
//! 只排，不碰界面：数由 `context_box.rs` 收好（[`Tail`]）。

use ratatui::text::Line;

use super::{Texts, bars};
use crate::core::{Split, UsageRow};
use crate::meter;
use crate::theme;
use crate::transcript::Stuck;

/// 上下文那一段底下要写的：压缩了几次、缓存断了几次、卡住了没有（照正在看的会话）。压缩线写在段头右边（第 3 条）。
#[derive(Debug, Clone, Copy, Default)]
pub struct Tail {
    pub compactions: u64,
    pub breaks: u64,
    pub stuck: Option<Stuck>,
}

/// 上下文那一段底下：一行暗字「已压缩 N 次 · 缓存断裂 N 次」；卡住的再一行红字说为什么。
pub(super) fn context_tail(tail: &Tail, texts: &Texts) -> Vec<Line<'static>> {
    let times = |n: u64| texts.times.replace("{n}", &n.to_string());
    let text = format!(
        "{} {}{}{} {}",
        texts.compactions,
        times(tail.compactions),
        crate::sheet::sep(),
        texts.breaks,
        times(tail.breaks)
    );
    let mut out = vec![Line::styled(text, theme::dim())];
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

/// 累计消耗的段头（右边缓存命中，收尾和下面的数一列）和四种 token 一种一根条（照多少从大到小排：数、占累计的百分比）；
/// 一次都没有的是空的。
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
    let counts = [
        split.cache_read,
        split.cache_write,
        split.output,
        split.uncached,
    ];
    let mut kinds: Vec<bars::Row> = texts
        .kinds
        .iter()
        .zip(counts)
        .map(|(name, n)| (0, name.clone(), n))
        .collect();
    // 多的在上（稳定排序：一样多的照缓存读、缓存写、输出、输入）。
    kinds.sort_by_key(|(_, _, n)| std::cmp::Reverse(*n));
    let laid = bars::rows(&kinds, all, room);
    let mut out = vec![bars::head(&title, &hit, laid.width)];
    out.extend(laid.lines);
    out
}
