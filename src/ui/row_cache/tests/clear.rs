//! 清屏（`Ctrl+L`、`/clear`）记的位置（蓝图「按键」`Ctrl+L`）。

use std::cell::RefCell;

use super::super::{Plan, RowCache, build};
use super::sample;
use crate::ui::test_support::Fixture;

#[test]
fn clearing_while_the_tail_spins_starts_the_new_screen_after_the_entries() {
    // 2026-10-15 macOS CI：`/clear` 在核心那边也开一轮，开轮以后正文末尾先转着几行（不属于哪一条）；这时画过一帧再到「清空」，
    // 清屏记下的行数把转着的那几行算了进去，这一轮收尾、那几行没了以后「上下文已清空」那一行落在新的一屏外面，整块空白。
    // 清屏只算属于条目的行。
    let f = Fixture::new();
    let t = sample();
    let ctx = f.ctx();
    let cache = RefCell::new(RowCache::default());
    let mut rows = build(&t.entries, &ctx, &cache, Plan::all());
    let entries = rows.len();
    rows.push(crate::ui::timeline::tail_rows(&ctx).into());
    assert!(rows.len() > entries, "末尾转着几行");
    let mut view = crate::body_view::BodyView::default();
    view.rows = rows;
    view.clear();
    assert_eq!(view.cleared_at, Some(entries), "转着的那几行不算");
}
