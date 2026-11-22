//! 长会话的正文只留视口附近几屏排好的行（施工 LW-22，蓝图「正文」第 8 条）：一帧三步。第一步 `row_cache::build` 照记着的
//! 拼出全部行，离视口远的只有行数；第二步这里的 [`place`] 照几何定这一帧的视口，把视口上下各 [`FILL_SCREENS`] 屏里只有
//! 行数的条目当场排出来；第三步画完以后 [`shed`] 把离视口超过 `keep_screens` 屏的放掉。

use std::cell::RefCell;
use std::ops::Range;

use ratatui::layout::Rect;

use crate::body_view::BodyView;
use crate::transcript::Entry;
use crate::ui::row_cache::{self, RowCache, Rows};
use crate::ui::rows::Ctx;

/// 补行的余量：视口上下各补几屏。比放掉的门槛（`layout.json` 的 `keep_screens`）小：留着的范围比马上要用的宽，来回滚
/// 的时候不反复排。
pub(super) const FILL_SCREENS: usize = 1;

/// 排这一帧要的：正文的条目、排版的条件、按条记着的行。
pub(super) struct Lay<'a> {
    pub entries: &'a [Entry],
    pub ctx: &'a Ctx<'a>,
    pub cache: &'a RefCell<RowCache>,
}

/// 第二步：定这一帧视口的第一行，画之前保证视口里每一行都有内容。`geometry` 照全部行的几何换算视口（`first_row` 和它
/// 前面那几样），在 `view` 的一份副本上算：补出来的行数和顶着的不一样（整份重排还在分帧做、只有旧行数顶着的）时
/// 照补好的重算一次视口、再补一次；最多两遍，第二遍以后视口不再动。点开、收起的那一条（锚点）、选区盖到的条目也补上：
/// 找锚点、复制要读它们的行。
pub(super) fn place(
    rows: &mut Rows,
    view: &mut BodyView,
    area: Rect,
    lay: &Lay,
    geometry: impl Fn(&Rows, &mut BodyView) -> usize,
) -> usize {
    let height = usize::from(area.height);
    let margin = FILL_SCREENS * height;
    let fill = |rows: &mut Rows, want: &[Range<usize>], also: &[usize]| {
        row_cache::fill(lay.entries, lay.ctx, lay.cache, rows, want, also)
    };
    // 锚点那一条：照它的第一行定视口，得先有行。
    let anchor: Vec<usize> = view
        .anchor
        .map(|(target, _)| row_cache::owner(target))
        .into_iter()
        .collect();
    fill(rows, &[], &anchor);
    let mut trial = view.clone();
    let mut first = geometry(rows, &mut trial);
    for pass in 0..2 {
        let near = first.saturating_sub(margin)..first + height + margin;
        let mut want = vec![near];
        want.extend(trial.held());
        if !fill(rows, &want, &[]).moved || pass == 1 {
            break;
        }
        trial = view.clone();
        first = geometry(rows, &mut trial);
    }
    // 兜底：第二遍补出来的又差了一屏以上时，视口落到的地方可能还有只剩行数的。视口不再动，补到它里面都有行为止
    // （每一遍至少补一块，块数有限）。
    let shown = first..first + height;
    while fill(rows, std::slice::from_ref(&shown), &[]).laid > 0 {}
    *view = trial;
    first
}

/// 第三步：画完以后，离视口超过 `keep_screens` 屏的条目把行放掉、只留行数；0 是全部留着。不放的：视口上下各
/// `keep_screens` 屏以内的、正文最末尾跟着最新的那几屏、选区盖到的、这一帧点开收起的那一条（`anchor`）；在进行的
/// 那几条缓存里本来就没有，`row_cache::shed_far` 不放。
pub(super) fn shed(
    rows: &mut Rows,
    view: &BodyView,
    (first, height): (usize, usize),
    keep_screens: usize,
    anchor: Option<usize>,
    lay: &Lay,
) {
    if keep_screens == 0 {
        return;
    }
    let reach = keep_screens.saturating_mul(height);
    let len = rows.len();
    let mut keep = vec![
        first.saturating_sub(reach)..first.saturating_add(height).saturating_add(reach),
        len.saturating_sub(reach.saturating_add(height))..len,
    ];
    keep.extend(view.held());
    let protect: Vec<usize> = anchor.into_iter().collect();
    row_cache::shed_far(lay.entries, lay.cache, rows, &keep, &protect);
}
