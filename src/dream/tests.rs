use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::*;

fn merging(upto: u64, fresh: u64, left: u64, given: &[&str]) -> Change {
    Change::Merging {
        upto,
        fresh,
        left,
        given: given.iter().map(|s| s.to_string()).collect(),
    }
}

fn texts(pairs: &[(&str, &str)]) -> Change {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    Change::Texts(map)
}

fn dream() -> (Dream, Instant) {
    let now = Instant::now();
    (Dream::new("s1".into(), now), now)
}

#[test]
fn batches_count_from_the_first_left_and_move_by_what_she_wrote_up_to_nine_tenths() {
    let (mut d, now) = dream();
    assert_eq!(d.batch(), None);
    d.apply(merging(12, 12, 48, &["m1", "m2"]), now);
    assert_eq!(d.batch(), Some((1, 4)));
    assert_eq!(d.progress(), (0, 12, 48));
    for _ in 0..20 {
        d.apply(Change::Revised { text: "改".into() }, now);
    }
    // 一批 12 条，写得再多也只挪到九成。
    assert_eq!(d.progress(), (10, 12, 48));
    d.apply(Change::Merged { upto: 12 }, now);
    assert_eq!(d.progress(), (12, 12, 48));
    d.apply(merging(24, 12, 36, &[]), now);
    assert_eq!(d.batch(), Some((2, 4)));
    assert_eq!(d.progress(), (12, 24, 48));
}

#[test]
fn a_failed_try_starts_the_batch_over_and_a_stray_merged_is_ignored() {
    let (mut d, now) = dream();
    d.apply(merging(12, 10, 10, &[]), now);
    d.apply(Change::Revised { text: "a".into() }, now);
    d.apply(Change::Revised { text: "b".into() }, now);
    assert_eq!(d.progress(), (2, 10, 10));
    d.apply(Change::Merged { upto: 99 }, now);
    assert_eq!(d.progress(), (2, 10, 10));
    d.apply(Change::Failed { upto: 12 }, now);
    assert_eq!(d.progress(), (0, 10, 10));
}

#[test]
fn only_reviewing_old_ones_counts_what_was_given() {
    let (mut d, now) = dream();
    d.apply(merging(5, 0, 0, &["m1", "m2", "m3"]), now);
    assert_eq!(d.progress(), (0, 3, 3));
    assert_eq!(d.batch(), Some((1, 1)));
}

#[test]
fn given_ones_float_by_once_then_stop_and_unknown_ones_are_skipped() {
    let (mut d, now) = dream();
    let every = Duration::from_millis(500);
    d.apply(texts(&[("m1", "黑咖啡"), ("m3", "猫咖")]), now);
    d.apply(merging(3, 3, 3, &["m1", "m2", "m3"]), now);
    d.tick(now, every);
    assert_eq!(
        d.rows().cloned().collect::<Vec<_>>(),
        [Line::Float("黑咖啡".into())]
    );
    // 没到点不浮。
    d.tick(now + Duration::from_millis(100), every);
    assert_eq!(d.rows().count(), 1);
    // m2 没有正文，跳过。
    d.tick(now + every, every);
    let rows: Vec<_> = d.rows().cloned().collect();
    assert_eq!(
        rows,
        [Line::Float("黑咖啡".into()), Line::Float("猫咖".into())]
    );
    assert_eq!(d.next_float(), None);
    d.tick(now + every * 10, every);
    assert_eq!(d.rows().count(), 2);
}

#[test]
fn the_first_thing_she_writes_stops_the_floating() {
    let (mut d, now) = dream();
    let every = Duration::from_millis(500);
    d.apply(texts(&[("m1", "一"), ("m2", "二")]), now);
    d.apply(merging(2, 2, 2, &["m1", "m2"]), now);
    d.tick(now, every);
    d.apply(
        Change::Retired {
            id: "m2".into(),
            why: Retire::Repeat,
        },
        now,
    );
    d.tick(now + every * 4, every);
    let rows: Vec<_> = d.rows().cloned().collect();
    assert_eq!(
        rows,
        [
            Line::Float("一".into()),
            Line::Retired("二".into(), Retire::Repeat)
        ]
    );
}

#[test]
fn the_summary_updates_one_line_in_place_and_the_area_keeps_five() {
    let (mut d, now) = dream();
    d.apply(merging(9, 9, 9, &[]), now);
    for i in 0..7 {
        d.apply(
            Change::Revised {
                text: format!("改{i}"),
            },
            now,
        );
    }
    d.apply(
        Change::Summary {
            chars: 20,
            done: false,
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
    d.apply(
        Change::Summary {
            chars: 52,
            done: true,
        },
        now,
    );
    let rows: Vec<_> = d.rows().cloned().collect();
    assert_eq!(rows.len(), ROWS);
    assert_eq!(rows.last(), Some(&Line::Summary(52, true)));
    assert_eq!(
        rows.iter()
            .filter(|l| matches!(l, Line::Summary(..)))
            .count(),
        1
    );
    assert_eq!(rows.first(), Some(&Line::Revised("改3".into())));
}

#[test]
fn retire_words_read_the_three_kinds() {
    assert_eq!(Retire::read("outdated"), Retire::Outdated);
    assert_eq!(Retire::read("repeat"), Retire::Repeat);
    assert_eq!(Retire::read("worthless"), Retire::Worthless);
    assert_eq!(Retire::read("?"), Retire::Worthless);
}

#[test]
fn stars_keep_a_gap_land_in_two_rows_and_light_in_a_shuffled_order() {
    let spots = place(40, 16, 5, 7);
    assert_eq!(spots.len(), 16);
    assert_eq!(spots, place(40, 16, 5, 7));
    let mut ranks: Vec<usize> = spots.iter().map(|s| s.rank).collect();
    assert_ne!(ranks, (0..16).collect::<Vec<_>>());
    ranks.sort_unstable();
    assert_eq!(ranks, (0..16).collect::<Vec<_>>());
    assert!(spots.iter().any(|s| s.row == 0) && spots.iter().any(|s| s.row == 1));
    for a in &spots {
        assert!(a.col < 40);
        for b in &spots {
            if a != b && a.row == b.row {
                assert!(a.col.abs_diff(b.col) >= 2, "{a:?} {b:?}");
            }
        }
    }
    assert!(place(0, 5, 5, 7).is_empty());
}
