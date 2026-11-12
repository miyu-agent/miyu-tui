//! 输入历史列表的高度定死、宽窗口预览放右边（蓝图 `tui.md`「输入历史列表」第 1 条「高度」「宽窗口预览放右边」，
//! 2026-11-12 项目主人，发版前 check 第 52 条：「Ctrl+R 打开的历史的列表有点太矮了……预览框默认只有一行，导致会因为内容
//! 上下来回变化高度」）。

use std::time::{Duration, SystemTime};

use ratatui::layout::Rect;
use serde_json::json;

use super::{Laid, Room, draw, index_at, lines, list_rows};
use crate::config::Config;
use crate::history::History;
use crate::input::{Draft, Sent};
use crate::theme;

/// 30 条：单数的一行，双数的三行（原来露着的几条长短不一，预览就一伸一缩）。
fn many(now: SystemTime) -> Vec<Sent> {
    (0..30)
        .map(|n| Sent {
            draft: Draft::plain(&if n % 2 == 0 {
                format!("第 {n} 条\n第二行\n第三行")
            } else {
                format!("第 {n} 条")
            }),
            at: now - Duration::from_secs(60 * n),
        })
        .collect()
}

fn room(width: u16, screen: u16, total: usize) -> Room {
    Room {
        width,
        max: usize::MAX,
        screen,
        total,
    }
}

fn plain(laid: &Laid) -> Vec<String> {
    laid.rows
        .iter()
        .map(|(_, l)| l.to_string().trim_end().to_string())
        .collect()
}

#[test]
fn the_box_keeps_its_height_while_moving_and_filtering() {
    let _theme = theme::hold();
    let config = Config::builtin().unwrap();
    let now = SystemTime::now();
    let list = many(now);
    let all: Vec<&Sent> = list.iter().collect();
    let at = room(60, 60, all.len());
    let mut history = History::default();
    let first = lines(&history, &all, at, &config, now).rows.len();
    // 窗口 60 行：列表 60 / 2 − 6 − 6 = 18 条，预览固定 6 行连框 8 行，横线、搜索两行。
    assert_eq!(first, 18 + 8 + 2);
    for _ in 0..12 {
        history.older(all.len());
        let laid = lines(&history, &all, at, &config, now);
        assert_eq!(laid.rows.len(), first, "上下选框不变：{:?}", plain(&laid));
    }
    // 筛到只剩两条：框照旧，两条贴着底，上面空着。
    let few = [all[0], all[1]];
    let history = History {
        query: "条".into(),
        ..History::default()
    };
    let laid = lines(&history, &few, at, &config, now);
    let text = plain(&laid);
    assert_eq!(laid.rows.len(), first, "{text:?}");
    assert!(text[..16].iter().all(String::is_empty), "{text:?}");
    assert!(
        text[17].starts_with("❯ ") && text[17].contains("第 0 条"),
        "{text:?}"
    );
    // 一条都对不上：「没有对得上的」贴着底，预览空着占位。
    let laid = lines(&history, &[], at, &config, now);
    let text = plain(&laid);
    assert_eq!(laid.rows.len(), first, "{text:?}");
    assert!(text[17].ends_with(&config.text.history.empty), "{text:?}");
}

#[test]
fn the_list_follows_the_window_unless_the_setting_gives_a_number() {
    let mut config = Config::builtin().unwrap();
    let layout = &config.layout;
    assert_eq!(layout.history_rows, 0, "出厂跟窗口高");
    // 上下排：窗口行数 / 2 − 预览 6 行 − 6，限在 8 到 20。
    assert_eq!(list_rows(layout, 100, 80, 40), 8);
    assert_eq!(list_rows(layout, 100, 80, 50), 13);
    assert_eq!(list_rows(layout, 100, 80, 100), 20);
    // 预览在右边：窗口行数 / 2 − 4。
    assert_eq!(list_rows(layout, 100, 120, 40), 16);
    assert_eq!(list_rows(layout, 100, 120, 100), 20);
    // 历史比这少的照历史的条数，没有历史的留一行写「没有对得上的」。
    assert_eq!(list_rows(layout, 3, 80, 60), 3);
    assert_eq!(list_rows(layout, 0, 80, 60), 1);
    // 两个配置项：写了数照数。
    config.layout.apply_history(&json!({"items": {
        "tui.history_rows": {"value": 5},
        "tui.history_preview_rows": {"value": 3},
    }}));
    assert_eq!(list_rows(&config.layout, 100, 80, 60), 5);
    let now = SystemTime::now();
    let list = many(now);
    let all: Vec<&Sent> = list.iter().collect();
    let laid = lines(&History::default(), &all, room(80, 60, 30), &config, now);
    assert_eq!(laid.rows.len(), 5 + (3 + 2) + 2, "{:?}", plain(&laid));
    // 没写的照原样。
    config.layout.apply_history(&json!({"items": {}}));
    assert_eq!(config.layout.history_rows, 5);
}

#[test]
fn a_wide_box_puts_the_preview_on_the_right() {
    let _theme = theme::hold();
    let config = Config::builtin().unwrap();
    let now = SystemTime::now();
    let list = many(now);
    let all: Vec<&Sent> = list.iter().collect();
    // 框里 120 列：左边列表 54 列（45%），竖线，空一格，右边写选中那一条的全文；列表和预览都是 60 / 2 − 4 = 20 行。
    let laid = lines(&History::default(), &all, room(120, 60, 30), &config, now);
    let text = plain(&laid);
    let side = laid.side.expect("宽框预览在右边");
    assert_eq!((side.col, side.rows, side.picked), (54, 20, Some(0)));
    assert_eq!(laid.rows.len(), 20 + 2, "{text:?}");
    // 竖线前面正好 54 列（按显示宽度量：时间是中文）。
    let split = |k: usize| {
        let spans = &laid.rows[k].1.spans;
        let at = spans
            .iter()
            .position(|s| s.content == "│ ")
            .expect("有竖线");
        let left: usize = spans[..at].iter().map(ratatui::text::Span::width).sum();
        let right: String = spans[at + 1..].iter().map(|s| s.content.as_ref()).collect();
        (left, right)
    };
    assert!((0..20).all(|k| split(k).0 == 54), "{text:?}");
    let right = |k: usize| split(k).1;
    assert_eq!(right(0), "第 0 条");
    assert_eq!(right(1), "第二行");
    assert_eq!(right(2), "第三行");
    assert!(text[19].starts_with("❯ ") && text[19].contains("第 0 条"));
    // 选中的那一条只在左边铺底，右边不铺。
    let picked = &laid.rows[19].1;
    assert_eq!(picked.style, ratatui::style::Style::default());
    assert!(picked.spans.last().unwrap().style.bg != theme::picked_bar().bg);
    // 点右边算选中的那一条，点左边算那一行。
    let text_area = Rect::new(2, 10, 120, 22);
    assert_eq!(index_at(text_area, &laid, 2 + 60, 10), Some(0));
    assert_eq!(index_at(text_area, &laid, 2 + 3, 10), Some(19));
    assert_eq!(index_at(text_area, &laid, 2 + 60, 30), None, "横线");
    // 不够 100 列的照上下排。
    let narrow = lines(&History::default(), &all, room(99, 60, 30), &config, now);
    assert!(narrow.side.is_none());
}

#[test]
fn the_divider_joins_the_top_edge_and_the_rule() {
    let _theme = theme::hold();
    let config = Config::builtin().unwrap();
    let now = SystemTime::now();
    let list = many(now);
    let all: Vec<&Sent> = list.iter().collect();
    let laid = lines(&History::default(), &all, room(116, 40, 30), &config, now);
    let height = u16::try_from(laid.rows.len()).unwrap() + 2;
    let backend = ratatui::backend::TestBackend::new(120, height);
    let mut term = ratatui::Terminal::new(backend).unwrap();
    term.draw(|frame| {
        let outer = Rect::new(0, 0, 120, height);
        let text = Rect::new(2, 1, 116, height - 2);
        draw(frame, outer, text, laid.clone());
    })
    .unwrap();
    let buffer = term.backend().buffer();
    let x = 2 + 52;
    assert_eq!(buffer[(x, 0)].symbol(), "┬");
    assert_eq!(buffer[(x, 1)].symbol(), "│");
    assert_eq!(buffer[(x, height - 3)].symbol(), "┴");
    assert_eq!(buffer[(0, height - 3)].symbol(), "├");
}

#[test]
fn a_short_window_still_shrinks_the_side_box_and_keeps_the_search_row() {
    let config = Config::builtin().unwrap();
    let now = SystemTime::now();
    let list = many(now);
    let all: Vec<&Sent> = list.iter().collect();
    let tight = Room {
        max: 7,
        ..room(120, 60, 30)
    };
    let laid = lines(&History::default(), &all, tight, &config, now);
    let text = plain(&laid);
    assert_eq!(laid.rows.len(), 7, "{text:?}");
    assert_eq!(laid.side.map(|s| s.rows), Some(5));
    assert!(text[4].starts_with("❯ "), "{text:?}");
    assert!(
        text[6].ends_with(&config.text.history.placeholder),
        "{text:?}"
    );
}
