use std::collections::HashMap;
use std::time::{Duration, Instant};

use ratatui::style::Modifier;
use unicode_width::UnicodeWidthStr;

use super::*;

/// 排成的行：头一行是和正文之间的空行，去掉它再数。
fn rows(d: &Dream, ctx: &Ctx, now: Instant) -> Vec<Row> {
    let mut got = super::rows(d, ctx, now);
    assert_eq!(text(&got[0]).trim(), "", "和上面空一行");
    got.remove(0);
    got
}
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
    // 她写出东西了：末尾只写整理了多久（第 3 条）。
    assert_eq!(got[0], "  ✦ 整理记忆 · 第 1 批 / 共 4 批 · 0s");
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
    // 一批之内不装进度：这一批没收尾，右下角还是 0/48。
    assert!(got[8].ends_with("0/48"), "{}", got[8]);
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

#[test]
fn before_she_writes_the_head_says_dreaming_and_the_clock_runs() {
    let f = Fixture::new();
    let now = Instant::now();
    let mut d = Dream::new("s1".into(), now);
    d.apply(
        Change::Merging {
            upto: 9,
            fresh: 0,
            left: 0,
            given: vec!["m1".into()],
        },
        now,
    );
    let later = now + Duration::from_secs(80);
    assert_eq!(
        text(&rows(&d, &f.ctx(), later)[0]),
        "  ✦ 整理记忆 · 第 1 批 / 共 1 批 · 做梦中… 1m 20s"
    );
    // 醒了：停在醒的那一刻，不再写做梦中。
    d.woke = Some((now + Duration::from_secs(90), "整理完成".into()));
    let head = text(&rows(&d, &f.ctx(), now + Duration::from_secs(99))[0]);
    assert!(head.ends_with("· 1m 30s"), "{head}");
}

#[test]
fn the_batch_stars_breathe_while_she_dreams() {
    let f = Fixture::new();
    let now = Instant::now();
    let look = &f.config.layout.dream;
    let mut d = Dream::new("s1".into(), now);
    d.apply(
        Change::Merging {
            upto: 9,
            fresh: 9,
            left: 9,
            given: Vec::new(),
        },
        now,
    );
    let soon = |at: Instant| {
        rows(&d, &f.ctx(), at)[ROWS + 2..]
            .iter()
            .flat_map(|r| r.line.spans.clone())
            .find(|s| s.content == look.soon.as_str())
            .map(|s| s.style)
            .expect("这一批的星")
    };
    let half = Duration::from_millis(look.breathe_ms / 2);
    assert_eq!(soon(now), theme::faint(), "一个来回从最暗起");
    assert_ne!(soon(now + half), soon(now), "半个来回亮起来");
    assert_eq!(breathe(look.breathe_ms as f64 / 2000.0, look), 1.0);
}
