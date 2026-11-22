//! 链接卡片、mermaid 图回来了只重排排的时候问过账的条目（施工 LW-24）：原来每回来一张，整份排好的行都扔掉重排，连
//! 离视口远的条目记着的行数也没了。

use std::cell::RefCell;

use super::super::{Plan, RowCache, build};
use crate::core::Card;
use crate::transcript::{Kind, Transcript};
use crate::ui::test_support::{Fixture, fresh_rows};

fn card(title: &str) -> Option<Card> {
    Some(Card {
        title: title.into(),
        site: "a.dev".into(),
        ..Card::default()
    })
}

fn texts(rows: impl Iterator<Item = String>) -> Vec<String> {
    rows.collect()
}

#[test]
fn a_card_arriving_rebuilds_only_entries_that_asked_the_book() {
    let f = Fixture::new();
    let mut t = Transcript::default();
    for n in 0..40 {
        t.note(Kind::Reply, format!("第 {n} 条，没有链接。"));
    }
    t.note(Kind::Reply, "看这个：\nhttps://a.dev/x".into());
    t.note(Kind::Reply, "最后一条。".into());
    let ctx = f.ctx();
    let cache = RefCell::new(RowCache::default());
    build(&t.entries, &ctx, &cache, Plan::all());
    assert_eq!(cache.borrow().rebuilt, 42, "头一帧全排");
    // 排的时候问过了：单子上有这个网址。
    assert_eq!(f.cards.borrow_mut().take().0, ["https://a.dev/x"]);
    build(&t.entries, &ctx, &cache, Plan::all());
    assert_eq!(cache.borrow().rebuilt, 0, "账没变，一条都不排");

    f.cards
        .borrow_mut()
        .got_card("https://a.dev/x".into(), card("A 的文章"));
    let rows = build(&t.entries, &ctx, &cache, Plan::all());
    assert_eq!(cache.borrow().rebuilt, 1, "只排带链接的那一条");
    let shown = texts(rows.present().map(|r| r.line.to_string()));
    assert!(shown.iter().any(|l| l.contains("A 的文章")), "{shown:#?}");
    // 和整份重排排出来的一样。
    let fresh = fresh_rows(&t.entries, &ctx);
    assert_eq!(shown, texts(fresh.iter().map(|r| r.line.to_string())));

    // 别的网址的卡片回来了：问过账的条目跟着排一遍（账只记变过几次，不分是哪个网址），没问过的照旧不排。
    f.cards
        .borrow_mut()
        .got_card("https://b.dev/y".into(), card("B"));
    build(&t.entries, &ctx, &cache, Plan::all());
    assert_eq!(cache.borrow().rebuilt, 1);
}

#[test]
fn losing_the_core_asks_again_for_what_never_came_back() {
    // 核心断开时没回来的卡片忘掉了：排好的行里记着占位，问过账的条目要重排一遍才会再问（`app/lost.rs`）。
    let f = Fixture::new();
    let mut t = Transcript::default();
    t.note(Kind::Reply, "没有链接。".into());
    t.note(Kind::Reply, "https://a.dev/x".into());
    let ctx = f.ctx();
    let cache = RefCell::new(RowCache::default());
    build(&t.entries, &ctx, &cache, Plan::all());
    assert_eq!(f.cards.borrow_mut().take().0, ["https://a.dev/x"]);
    f.cards.borrow_mut().lost();
    build(&t.entries, &ctx, &cache, Plan::all());
    assert_eq!(cache.borrow().rebuilt, 1, "只排问过账的那一条");
    assert_eq!(
        f.cards.borrow_mut().take().0,
        ["https://a.dev/x"],
        "又记进了单子"
    );
}

#[test]
fn a_diagram_arriving_rebuilds_only_the_entry_that_drew_it() {
    let f = Fixture::new();
    let mut t = Transcript::default();
    t.note(Kind::Reply, "没有图。".into());
    t.note(Kind::Reply, "```mermaid\ngraph TD\n  A --> B\n```".into());
    // 能显示图的终端：mermaid 才去问核心要图。
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
    build(&t.entries, &ctx, &cache, Plan::all());
    let asked = f.diagrams.borrow_mut().take();
    assert_eq!(asked.len(), 1, "问了一张：{asked:?}");
    build(&t.entries, &ctx, &cache, Plan::all());
    assert_eq!(cache.borrow().rebuilt, 0);
    // 核心拒了（没装画图的）：账变了一次，只排那一条。
    f.diagrams.borrow_mut().got(asked[0].clone(), None);
    build(&t.entries, &ctx, &cache, Plan::all());
    assert_eq!(cache.borrow().rebuilt, 1, "只排带 mermaid 的那一条");
}
