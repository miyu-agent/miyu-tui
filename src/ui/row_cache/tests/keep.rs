//! 离视口远的条目只留行数（施工 LW-22，蓝图「正文」第 8 条）：放掉以后几何和全部留着时一样；没变的不重排、照旧只有
//! 行数；补行现排出来的和整份排的一样；指纹变了的先用旧行数顶着、分帧排完和整份排的一样；在进行的不放；带图的放掉
//! 以后图做好了，排回来占的行数跟着变。

use std::cell::RefCell;
use std::time::Duration;

use super::super::keep::Filled;
use super::super::{Plan, RowCache, Rows, build, fill, shed_far};
use super::sample;
use crate::transcript::{Kind, Transcript};
use crate::ui::rows::Ctx;
use crate::ui::test_support::{Fixture, fresh_rows};

/// `copies` 份 [`sample`] 接起来，每一条一个编号。
fn long(copies: usize) -> Transcript {
    let mut t = Transcript::default();
    for _ in 0..copies {
        t.entries.extend(sample().entries);
    }
    for (i, e) in t.entries.iter_mut().enumerate() {
        e.id = i as u64;
    }
    t
}

/// 只有一段行号（放进要几段的地方）。
fn only(rows: std::ops::Range<usize>) -> Vec<std::ops::Range<usize>> {
    vec![rows]
}

/// 每一条从第几行到第几行。
fn spans(rows: &Rows, n: usize) -> Vec<(Option<usize>, Option<usize>)> {
    (0..n).map(|i| (rows.start_of(i), rows.end_of(i))).collect()
}

/// 第 `from` 行起 `n` 行的字，放掉了的是 `None`。
fn texts(rows: &Rows, from: usize, n: usize) -> Vec<Option<String>> {
    (from..from + n)
        .map(|i| rows.get(i).map(|r| r.line.to_string()))
        .collect()
}

/// 整份排出来的第 `from` 行起 `n` 行的字。
fn fresh_texts(
    entries: &[crate::transcript::Entry],
    ctx: &Ctx,
    from: usize,
    n: usize,
) -> Vec<Option<String>> {
    let fresh = fresh_rows(entries, ctx);
    fresh[from..from + n]
        .iter()
        .map(|r| Some(r.line.to_string()))
        .collect()
}

#[test]
fn far_entries_keep_only_their_length_and_come_back_when_filled() {
    let f = Fixture::new();
    let t = long(6);
    let n = t.entries.len();
    let ctx = f.ctx();
    let cache = RefCell::new(RowCache::default());
    let mut rows = build(&t.entries, &ctx, &cache, Plan::all());
    let whole = spans(&rows, n);
    let total = rows.len();
    // 只留最后 10 行：前面的条目放掉行，只留行数。
    shed_far(&t.entries, &cache, &mut rows, &only(total - 10..total), &[]);
    assert_eq!(rows.kept(0), Some(false), "离得远的放掉了");
    assert_eq!(rows.kept(n - 1), Some(true), "留着的范围里的还在");
    assert!(rows.get(0).is_none(), "放掉的取不到行");
    assert_eq!(
        (rows.len(), spans(&rows, n)),
        (total, whole.clone()),
        "几何不变"
    );
    // 下一帧什么都没变：一条都不排，放掉的照旧只有行数。
    let mut again = build(&t.entries, &ctx, &cache, Plan::all());
    assert_eq!(cache.borrow().rebuilt, 0, "只有行数的、指纹没变的不排");
    assert_eq!(again.kept(0), Some(false));
    assert_eq!((again.len(), spans(&again, n)), (total, whole));
    // 补第一条：现排出来的和整份排的一样，行数没变。
    let first = again.end_of(0).unwrap();
    let filled = fill(&t.entries, &ctx, &cache, &mut again, &only(0..1), &[]);
    assert_eq!(
        filled,
        Filled {
            laid: 1,
            moved: false
        }
    );
    assert_eq!(
        texts(&again, 0, first),
        fresh_texts(&t.entries, &ctx, 0, first)
    );
    assert_eq!(again.kept(1), Some(false), "只补要的那几行落在的");
}

#[test]
fn entries_still_in_progress_are_never_shed() {
    let f = Fixture::new();
    let mut t = long(3);
    // 第二条是一段还在跑的时间线（每帧重排，缓存里没有）。
    if let Some(seg) = t.entries[1].segment.as_mut() {
        seg.finished = false;
    }
    let ctx = f.ctx();
    let cache = RefCell::new(RowCache::default());
    let mut rows = build(&t.entries, &ctx, &cache, Plan::all());
    // 末尾转圈的那几行不属于哪一条。
    let tail = vec![ctx.row(ctx.blank_slot(), Vec::new()); 2];
    rows.push(tail.into());
    let last = rows.len() - 1;
    shed_far(&t.entries, &cache, &mut rows, &[], &[]);
    assert_eq!(rows.kept(1), Some(true), "在进行的不放");
    assert_eq!(rows.kept(0), Some(false), "做完了的照放");
    assert!(rows.get(last).is_some(), "末尾转圈的那几行不放");
    // 点开收起的那一条（锚点）、选区盖到的不放。
    let mut rows = build(&t.entries, &ctx, &cache, Plan::all());
    let len = rows.len();
    fill(&t.entries, &ctx, &cache, &mut rows, &only(0..len), &[]);
    let at = rows.start_of(4).unwrap();
    shed_far(&t.entries, &cache, &mut rows, &only(at..at + 1), &[2]);
    assert_eq!(rows.kept(2), Some(true), "锚点那一条不放");
    assert_eq!(rows.kept(4), Some(true), "盖到的那一条不放");
    assert_eq!(rows.kept(3), Some(false));
}

#[test]
fn a_shed_entry_whose_layout_changed_waits_with_its_old_length_then_matches_a_fresh_build() {
    // 改宽度以后整份重排分帧做：只有行数的和有旧行的一样，这一帧先用旧行数顶着、算进 `stale`；补行现排了的不再算；
    // 排完和整份排的一模一样。
    let f = Fixture::new();
    let t = long(5);
    let n = t.entries.len();
    let mut ctx = f.ctx();
    let cache = RefCell::new(RowCache::default());
    let mut rows = build(&t.entries, &ctx, &cache, Plan::all());
    let total = rows.len();
    shed_far(&t.entries, &cache, &mut rows, &only(total - 5..total), &[]);
    let old = spans(&rows, n);
    drop(rows);
    ctx.width = 40;
    let tight = Plan {
        budget: Some(Duration::ZERO),
        anchor: None,
    };
    let mut rows = build(&t.entries, &ctx, &cache, tight);
    assert_eq!(cache.borrow().stale, n - 1, "只有行数的也先顶着");
    assert_eq!(rows.kept(0), Some(false));
    assert_eq!(rows.start_of(1), old[1].0, "顶着的是旧行数");
    let filled = fill(&t.entries, &ctx, &cache, &mut rows, &only(0..1), &[]);
    assert_eq!(filled.laid, 1);
    assert_eq!(cache.borrow().stale, n - 2, "现排了的不再算没排完");
    let mut frames = 0;
    while cache.borrow().stale > 0 {
        build(&t.entries, &ctx, &cache, tight);
        frames += 1;
        assert!(frames <= n, "每帧至少排一条");
    }
    let mut done = build(&t.entries, &ctx, &cache, tight);
    let len = done.len();
    fill(&t.entries, &ctx, &cache, &mut done, &only(0..len), &[]);
    let fresh = fresh_rows(&t.entries, &ctx);
    assert_eq!(done.len(), fresh.len());
    let all: Vec<String> = done.present().map(|r| r.line.to_string()).collect();
    assert_eq!(
        all,
        fresh.iter().map(|r| r.line.to_string()).collect::<Vec<_>>()
    );
}

#[test]
fn a_shed_entry_with_a_figure_comes_back_with_the_figure_rows() {
    // 带图的条目放掉以后图做好了（占几行变了）：指纹变了，这一帧先用旧行数顶着；补行现排出来占的是图的行数，后面的
    // 跟着挪，和整份排的一样。
    let f = Fixture::new();
    let mut t = Transcript::default();
    t.note(Kind::Reply, "$$\nx^2 + y^2\n$$".into());
    t.entries.extend(long(2).entries);
    for (i, e) in t.entries.iter_mut().enumerate() {
        e.id = i as u64;
    }
    let (figures, jobs) = crate::figures::queued(8);
    let figures = RefCell::new(figures);
    let mut ctx = f.ctx();
    ctx.figures = &figures;
    let cache = RefCell::new(RowCache::default());
    let mut rows = build(&t.entries, &ctx, &cache, Plan::all());
    let pending = rows.end_of(0).unwrap();
    let total = rows.len();
    shed_far(&t.entries, &cache, &mut rows, &only(total - 3..total), &[]);
    assert_eq!(rows.kept(0), Some(false), "还在画的那一条也放掉了");
    drop(rows);
    crate::figures::finish(&mut figures.borrow_mut(), &jobs, 200, 6);
    // 同一帧末尾新来一条：从最底下往上排，预算先给它，带图的那一条没轮到。
    let mut more = Transcript::default();
    more.note(Kind::Reply, "新来的一条".into());
    let mut extra = more.entries.remove(0);
    extra.id = 1000;
    t.entries.push(extra);
    let tight = Plan {
        budget: Some(Duration::ZERO),
        anchor: None,
    };
    let mut rows = build(&t.entries, &ctx, &cache, tight);
    assert_eq!(cache.borrow().stale, 1, "带图的那一条先用旧行数顶着");
    assert_eq!((rows.kept(0), rows.end_of(0)), (Some(false), Some(pending)));
    let filled = fill(&t.entries, &ctx, &cache, &mut rows, &only(0..1), &[]);
    assert_eq!(
        filled,
        Filled {
            laid: 1,
            moved: true
        },
        "图做好了占的行变了"
    );
    assert_eq!(cache.borrow().stale, 0);
    let end = rows.end_of(0).unwrap();
    assert_eq!(
        end,
        fresh_rows(&t.entries[..1], &ctx).len(),
        "排回来占的是图的行数"
    );
    assert!(end > pending);
    assert_eq!(texts(&rows, 0, end), fresh_texts(&t.entries, &ctx, 0, end));
    assert_eq!(
        rows.len(),
        fresh_rows(&t.entries, &ctx).len(),
        "后面的跟着挪"
    );
}
