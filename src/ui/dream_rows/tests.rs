use std::collections::HashMap;
use std::time::{Duration, Instant};

use ratatui::style::Modifier;
use unicode_width::UnicodeWidthStr;

use super::*;
use crate::dream::Change;
use crate::ui::test_support::Fixture;

fn text(row: &Row) -> String {
    row.line.spans.iter().map(|s| s.content.as_ref()).collect()
}

fn dreaming(now: Instant) -> Dream {
    let mut d = Dream::new("s1".into(), now);
    let texts: HashMap<String, String> = [("m1", "对猫毛不过敏"), ("m2", "周末常去附近的猫咖")]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    d.apply(Change::Texts(texts), now);
    d.apply(
        Change::Merging {
            upto: 12,
            fresh: 12,
            left: 48,
            given: vec!["m2".into()],
        },
        now,
    );
    d.tick(now, Duration::from_millis(500));
    d.apply(
        Change::Retired {
            id: "m1".into(),
            why: Retire::Repeat,
        },
        now,
    );
    d.apply(
        Change::Revised {
            text: "75 配列，红轴，用了一个月".into(),
        },
        now,
    );
    d.apply(
        Change::Summary {
            chars: 40,
            done: false,
        },
        now,
    );
    d
}

#[test]
fn the_block_is_a_head_five_lines_a_gap_and_two_sky_rows() {
    let f = Fixture::new();
    let now = Instant::now();
    let got: Vec<String> = rows(&dreaming(now), &f.ctx(), now)
        .iter()
        .map(text)
        .collect();
    assert_eq!(got.len(), 1 + ROWS + 1 + 2, "{got:#?}");
    assert_eq!(got[0], "  ✦ 整理记忆 · 第 1 批 / 共 4 批");
    // 滚动区不够 5 行的上面空着，新的在下面。
    assert_eq!(got[1].trim(), "");
    assert_eq!(got[2], "  · 周末常去附近的猫咖");
    assert_eq!(got[3], "  ✗ 对猫毛不过敏 · 重复");
    assert_eq!(got[4], "  ✎ 75 配列，红轴，用了一个月");
    assert_eq!(got[5], "  ✦ 摘要 40 字…");
    assert_eq!(got[6].trim(), "");
    assert!(
        got[8].starts_with("  ") && got[8].contains("(－ω－)"),
        "{}",
        got[8]
    );
    // 写了删、改两条（摘要不算）：一批之内照写出的条数挪，右下角写合了 2/48。
    assert!(got[8].ends_with("2/48"), "{}", got[8]);
}

#[test]
fn only_the_marks_carry_colors_and_the_deleted_text_is_struck_through() {
    let f = Fixture::new();
    let now = Instant::now();
    let got = rows(&dreaming(now), &f.ctx(), now);
    let span = |row: usize, text: &str| {
        got[row]
            .line
            .spans
            .iter()
            .find(|s| s.content.contains(text))
            .unwrap()
            .style
    };
    assert_eq!(span(2, "猫咖"), theme::dim());
    assert_eq!(span(3, "✗"), theme::removed());
    assert!(
        span(3, "对猫毛")
            .add_modifier
            .contains(Modifier::CROSSED_OUT)
    );
    assert_eq!(span(3, "重复"), theme::dim());
    assert_eq!(span(4, "✎"), theme::accent());
    assert_eq!(span(4, "红轴"), ratatui::style::Style::new());
    assert_eq!(span(5, "✦"), theme::good());
}

#[test]
fn long_lines_are_clipped_to_the_width_and_never_wrap() {
    let f = Fixture::new();
    let now = Instant::now();
    let mut d = Dream::new("s1".into(), now);
    d.apply(
        Change::Merging {
            upto: 1,
            fresh: 1,
            left: 1,
            given: Vec::new(),
        },
        now,
    );
    d.apply(
        Change::Revised {
            text: "很长的一条记忆".repeat(20),
        },
        now,
    );
    let ctx = f.ctx();
    let got = rows(&d, &ctx, now);
    let line = text(&got[ROWS]);
    assert!(line.ends_with('…'), "{line}");
    assert!(line.width() <= usize::from(ctx.width) + 2, "{line}");
    for row in &got {
        assert!(
            text(row).width() <= usize::from(ctx.width) + 2,
            "{}",
            text(row)
        );
    }
}

#[test]
fn stars_light_with_progress_and_all_light_when_she_wakes() {
    let f = Fixture::new();
    let now = Instant::now();
    let look = &f.config.layout.dream;
    let lit = |d: &Dream| -> usize {
        rows(d, &f.ctx(), now)[ROWS + 2..]
            .iter()
            .flat_map(|r| r.line.spans.iter())
            .filter(|s| {
                s.style.fg == theme::warn().fg && !s.style.add_modifier.contains(Modifier::DIM)
            })
            .filter(|s| look.glyphs.iter().any(|g| s.content == g.as_str()))
            .count()
    };
    let mut d = Dream::new("s1".into(), now);
    d.apply(
        Change::Merging {
            upto: 24,
            fresh: 24,
            left: 48,
            given: Vec::new(),
        },
        now,
    );
    assert_eq!(lit(&d), 0);
    d.apply(Change::Merged { upto: 24 }, now);
    let half = lit(&d);
    assert!(half > 0, "合了一半亮了一些");
    d.woke = Some((now, "整理完了".into()));
    let all = lit(&d);
    assert!(all > half, "{half} {all}");
    let face = text(&rows(&d, &f.ctx(), now)[ROWS + 3]);
    assert!(face.contains(&look.awake), "{face}");
}

#[test]
fn a_twinkling_star_brightens_only_now_and_then() {
    let f = Fixture::new();
    let look = &f.config.layout.dream;
    let samples: Vec<f64> = (0..100)
        .map(|i| twinkle(i as f64 * 0.05, 0, look))
        .collect();
    let bright = samples
        .iter()
        .filter(|v| **v > look.twinkle_lift / 2.0)
        .count();
    assert!(bright > 0 && bright < 30, "{bright}");
    assert!(
        samples
            .iter()
            .all(|v| (0.0..=look.twinkle_lift).contains(v))
    );
}
