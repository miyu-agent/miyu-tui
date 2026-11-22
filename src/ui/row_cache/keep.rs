//! 长会话里只留视口附近几屏排好的行（施工 LW-22，蓝图「正文」第 8 条）：画之前把要看的那几屏里只剩行数的条目当场排出来
//! （[`fill`]），画完以后离视口远的条目放掉行、只留行数（[`shed_far`]）。行数照旧记着，滚动条、跳转、视口换算这些只看
//! 几何的照样答得出；滚近了再排。

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use super::{RowCache, Rows};
use crate::transcript::Entry;
use crate::ui::rows::Ctx;

/// 补了没有：补了几块、有没有哪一块补出来的行数和顶着的不一样（行号跟着挪了）。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Filled {
    /// 现排了几块。
    pub laid: usize,
    /// 有一块现排出来的行数和原来顶着的不一样。
    pub moved: bool,
}

/// 补行：`rows` 里和 `want` 这几段行号有交叠的块、还有第 `also` 几条的块，只有行数没有行的，当场排出来补进 `rows` 和
/// 缓存，不受重排预算限制（视口附近的几条，条数很少）。指纹变了、这一帧先用旧行数顶着的（算在 `stale` 里），现排了就
/// 不再算。
pub fn fill(
    entries: &[Entry],
    ctx: &Ctx,
    cache: &RefCell<RowCache>,
    rows: &mut Rows,
    want: &[Range<usize>],
    also: &[usize],
) -> Filled {
    let mut todo: Vec<usize> = want
        .iter()
        .flat_map(|range| rows.chunks_in(range.clone()))
        .chain(also.iter().flat_map(|&entry| rows.chunks_of(entry)))
        .filter(|&k| {
            let (owner, _, kept) = rows.chunk(k);
            owner.is_some_and(|i| i < entries.len()) && kept.is_none()
        })
        .collect();
    // 从后往前换：换掉一块只挪它后面的行号，前面的块的下标不变。
    todo.sort_unstable();
    todo.dedup();
    let figures = ctx.figures.borrow().revision();
    let mut cache = cache.borrow_mut();
    let mut out = Filled::default();
    for k in todo.into_iter().rev() {
        let Some(i) = rows.chunk(k).0 else {
            continue;
        };
        let entry = &entries[i];
        let laid = cache.lay(i, entry, ctx, figures);
        if cache.stale_ids.remove(&entry.id) {
            cache.stale -= 1;
        }
        out.laid += 1;
        out.moved |= rows.replace(k, laid);
    }
    out
}

/// 放掉：`keep` 这几段行号以外的块、第 `protect` 几条以外的条目，把行放掉、只留行数（缓存里的和 `rows` 里的一起放，
/// 别处没人拿着就真还掉了）。只放缓存里记着的、和这一帧用的是同一份的：在进行的（指纹是 `None`、每帧重排的）缓存里
/// 没有，不放；条目间的空行、末尾转圈的那几行不属于哪一条，不放。
pub fn shed_far(
    entries: &[Entry],
    cache: &RefCell<RowCache>,
    rows: &mut Rows,
    keep: &[Range<usize>],
    protect: &[usize],
) {
    let mut cache = cache.borrow_mut();
    let mut shed = 0;
    for k in 0..rows.chunk_count() {
        let (owner, span, chunk) = rows.chunk(k);
        let (Some(i), Some(chunk)) = (owner, chunk) else {
            continue;
        };
        if protect.contains(&i)
            || keep
                .iter()
                .any(|r| r.start < span.end && span.start < r.end)
        {
            continue;
        }
        let Some(entry) = entries.get(i) else {
            continue;
        };
        let Some(kept) = cache.entries.get_mut(&entry.id) else {
            continue;
        };
        if kept.rows.as_ref().is_some_and(|r| Rc::ptr_eq(r, chunk)) {
            kept.rows = None;
            rows.drop_rows(k);
            shed += span.len();
        }
    }
    cache.shed += shed;
}
