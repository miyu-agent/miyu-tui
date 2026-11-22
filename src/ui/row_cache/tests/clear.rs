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

#[test]
fn shed_rows_keep_their_geometry_and_hand_out_nothing() {
    // 施工 LW-20：停放着的会话把上一帧的行放掉，切回来时照行号换算位置的那些问法答案不变。
    let f = Fixture::new();
    let t = sample();
    let ctx = f.ctx();
    let cache = RefCell::new(RowCache::default());
    let rows = build(&t.entries, &ctx, &cache, Plan::all());
    let mut shed = rows.clone();
    shed.shed();
    assert_eq!(shed.len(), rows.len());
    assert_eq!(shed.content_len(), rows.content_len());
    for row in 0..rows.len() {
        assert_eq!(
            shed.entry_at(row),
            rows.entry_at(row),
            "第 {row} 行是第几条的"
        );
    }
    for entry in 0..t.entries.len() {
        assert_eq!(shed.start_of(entry), rows.start_of(entry));
        assert_eq!(shed.end_of(entry), rows.end_of(entry));
    }
    assert!(rows.get(0).is_some());
    assert!(shed.get(0).is_none());
    assert_eq!(shed.iter().count(), 0);
    assert_eq!(shed.window(0, rows.len()).count(), 0);
}

#[test]
fn a_parked_view_lets_go_of_its_rows_and_selection_but_stays_where_it_was() {
    let f = Fixture::new();
    let t = sample();
    let ctx = f.ctx();
    let cache = RefCell::new(RowCache::default());
    let rows = build(&t.entries, &ctx, &cache, Plan::all());
    let total = rows.len();
    let mut view = crate::body_view::BodyView::default();
    view.rows = rows;
    view.top = Some(3);
    view.first = 3;
    view.select = Some(((1, 0), (2, 4)));
    view.park();
    assert_eq!(view.rows.len(), total, "一共几行还答得出");
    assert_eq!(view.rows.iter().count(), 0, "行放掉了");
    assert_eq!(view.select, None, "选中的一段指着的行不在了，放开");
    assert_eq!((view.top, view.first), (Some(3), 3), "停在哪照旧记着");
}
