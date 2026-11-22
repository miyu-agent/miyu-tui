//! 一行里要重写哪几截：变了的格子（活跃的帧，ratatui 逐格比出来的）或者变了的段（醒来的那一帧，照指纹比出来的），相邻的
//! 并成一截，两头往外扩到能单独写的地方。
//!
//! 往外扩的四条（扩出去的格子内容没变，不用重新记）：
//! 1. 截的第一格是宽字盖住的后半：往左扩到那个宽字，从它写起（不然从后半写起会把宽字劈掉）。
//! 2. 截的最后一格是宽字的前半、后半落在截外：往右扩到盖住的最后一格。
//! 3. 截的左邻格写着宽度认不准的字：往左扩一格把它带上（`glyph_guard` 开头第 3 条：它右边那一格变了，它也要重写）。
//! 4. 截的最后一格写着宽度认不准的字，或者上一帧写着（换成了别的）：往右扩一格把它右边那格带上（同一条：这种字变了，
//!    右边那一格跟着重写）。
//!
//! 扩完和别的截挨上、叠上的并成一截。四条照顺序反复用到不再变。

use ratatui::buffer::{Cell, CellWidth};

use super::SEGMENT;
use crate::glyph_guard;

/// 变了的段（`dirty` 是每一段变没变）连成的几截，`[起, 止)` 两个列号：醒来的那一帧照指纹比出来的。
pub fn segments(dirty: &[bool], width: usize) -> Vec<(usize, usize)> {
    let mut raw: Vec<(usize, usize)> = Vec::new();
    for (k, _) in dirty.iter().enumerate().filter(|(_, d)| **d) {
        join(&mut raw, k * SEGMENT, ((k + 1) * SEGMENT).min(width));
    }
    raw
}

/// 变了的格子（列号，从小到大）连成的几截：活跃的帧照 ratatui 逐格比出来的。
pub fn cells(xs: impl IntoIterator<Item = usize>) -> Vec<(usize, usize)> {
    let mut raw: Vec<(usize, usize)> = Vec::new();
    for x in xs {
        join(&mut raw, x, x + 1);
    }
    raw
}

/// `[start, end)` 接在最后一截后面的并进去，不挨着的另起一截。
fn join(raw: &mut Vec<(usize, usize)>, start: usize, end: usize) {
    match raw.last_mut() {
        Some(last) if last.1 >= start => last.1 = last.1.max(end),
        _ => raw.push((start, end)),
    }
}

/// 这一行（`row`）里要重写的几截，每截是 `[起, 止)` 两个列号：`raw` 是变了的几截（从左到右），照开头的四条往外扩。
/// `was` 说这一列上一帧写没写着宽度认不准的字。
pub fn spans(
    row: &[Cell],
    raw: Vec<(usize, usize)>,
    was: impl Fn(usize) -> bool,
) -> Vec<(u16, u16)> {
    let width = row.len();
    if raw.is_empty() {
        return Vec::new();
    }
    let owner = owners(row);
    let shaky = |x: usize| glyph_guard::shaky(row[x].symbol());
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (mut start, mut end) in raw {
        loop {
            let before = (start, end);
            // 第 1 条。
            start = owner[start];
            // 第 3 条。
            if start > 0 && shaky(owner[start - 1]) {
                start = owner[start - 1];
            }
            // 第 2 条。
            let lead = owner[end - 1];
            end = end.max((lead + span_of(&row[lead])).min(width));
            // 第 4 条。
            if end < width && (shaky(owner[end - 1]) || was(end - 1)) {
                end += 1;
            }
            if (start, end) == before {
                break;
            }
        }
        match out.last_mut() {
            Some(last) if start <= last.1 => *last = (last.0.min(start), last.1.max(end)),
            _ => out.push((start, end)),
        }
    }
    // 一行的宽本来就是 u16，列号都转得回去。
    out.into_iter().map(|(s, e)| (s as u16, e as u16)).collect()
}

/// 每一格归哪一格写：宽字盖住的后面那几格归那个宽字，别的归自己。规矩同 ratatui 的 `Buffer::diff`（图占着的格子只占
/// 自己那一格，宽字、强定了宽度的盖住后面几格）。
pub fn owners(row: &[Cell]) -> Vec<usize> {
    let mut owner = vec![0; row.len()];
    let mut x = 0;
    while x < row.len() {
        let cell = &row[x];
        let span = if glyph_guard::skipped(cell) {
            1
        } else {
            span_of(cell)
        };
        for slot in &mut owner[x..(x + span).min(row.len())] {
            *slot = x;
        }
        x += span;
    }
    owner
}

/// 这一格写下去占几格（至少一格）。
fn span_of(cell: &Cell) -> usize {
    usize::from(cell.cell_width().max(1))
}

/// 这一格写下去盖到右边第几格（不含）为止，从它自己算起：图占着的格子只算自己。
pub fn reach(cell: &Cell) -> u16 {
    if glyph_guard::skipped(cell) {
        1
    } else {
        cell.cell_width().max(1)
    }
}
