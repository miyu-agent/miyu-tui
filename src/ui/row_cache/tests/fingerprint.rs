//! 指纹漏掉的排版条件（review C4、C5，2026-10-11）：变了要重排的，数重排了几条。

use std::cell::RefCell;

use super::super::{Plan, RowCache, build};
use crate::transcript::{Kind, Transcript};
use crate::ui::test_support::Fixture;

#[test]
fn only_the_screen_height_changing_rebuilds_entries_with_figures() {
    // review C4（2026-10-11）：图占几行照屏高算（三分之一屏），只拉高、压矮窗口时按条缓存的行照旧，图不重算。
    let _theme = crate::theme::hold();
    let f = Fixture::new();
    let mut t = Transcript::default();
    t.note(Kind::Reply, "$$\nx^2 + y^2\n$$".into());
    t.note(Kind::Reply, "没有图。".into());
    // 能显示图的终端：块级公式排成图（先是占位）。
    let mut picker = ratatui_image::picker::Picker::halfblocks();
    picker.set_protocol_type(ratatui_image::picker::ProtocolType::Kitty);
    let graphics = Some(crate::figures::Graphics { picker });
    let kitty = RefCell::new(crate::figures::Figures::start(
        graphics,
        &f.config.figures,
        None,
        |_| true,
    ));
    let mut ctx = f.ctx();
    ctx.figures = &kitty;
    let cache = RefCell::new(RowCache::default());
    let rows = build(&t.entries, &ctx, &cache, Plan::all());
    assert!(
        rows.iter().any(|r| r.figure.is_some() || r.figure_pending),
        "第一条带图"
    );
    ctx.screen_rows += 20;
    build(&t.entries, &ctx, &cache, Plan::all());
    assert_eq!(cache.borrow().rebuilt, 1, "只排带图的那一条");
}

#[test]
fn files_arriving_for_the_undo_line_rebuild_it() {
    // review C5（2026-10-11）：撤销的回应和改回的文件是两个事件，中间画过一帧的话「改回 N 个文件」一直不出现。
    let _theme = crate::theme::hold();
    let f = Fixture::new();
    let mut t = Transcript::default();
    t.note(Kind::Undo, "已撤销".into());
    t.entries[0].undo = Some(crate::core::Report::default());
    let ctx = f.ctx();
    let cache = RefCell::new(RowCache::default());
    build(&t.entries, &ctx, &cache, Plan::all());
    let file = crate::core::UndoFile {
        path: "/w/a.rs".into(),
        outcome: "restored".into(),
        ..crate::core::UndoFile::default()
    };
    if let Some(undo) = t.entries[0].undo.as_mut() {
        undo.files = vec![file];
    }
    build(&t.entries, &ctx, &cache, Plan::all());
    assert_eq!(cache.borrow().rebuilt, 1);
}
