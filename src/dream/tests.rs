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
fn batches_count_from_the_first_left_and_a_batch_counts_only_once_it_is_merged() {
    let (mut d, now) = dream();
    assert_eq!(d.batch(), None);
    d.apply(merging(12, 12, 48, &["m1", "m2"]), now);
    assert_eq!(d.batch(), Some((1, 4)));
    assert_eq!(d.progress(), (0, 12, 48));
    // 一批之内不装进度（2026-10-20 项目主人：她想完一口气写完，按写出的条数挪是假的）。
    for _ in 0..20 {
        d.apply(Change::Revised { text: "改".into() }, now);
    }
    assert_eq!(d.progress(), (0, 12, 48));
    d.apply(Change::Merged { upto: 12 }, now);
    assert_eq!(d.progress(), (12, 12, 48));
    d.apply(merging(24, 12, 36, &[]), now);
    assert_eq!(d.batch(), Some((2, 4)));
    assert_eq!(d.progress(), (12, 24, 48));
}

#[test]
fn what_landed_after_merging_began_is_kept_with_retired_texts_filled_in() {
    let (mut d, now) = dream();
    d.apply(texts(&[("m1", "住在上海")]), now);
    // 开始合以前的（会话里先抽的）不算。
    d.apply(Change::Landed(Landed::Revised("先抽的".into())), now);
    d.apply(merging(3, 3, 3, &["m1"]), now);
    d.apply(
        Change::Landed(Landed::Retired("m1".into(), Retire::Outdated)),
        now,
    );
    d.apply(Change::Landed(Landed::Revised("搬到杭州了".into())), now);
    d.apply(Change::Landed(Landed::Summary("用户住杭州。".into())), now);
    assert_eq!(
        d.kept(),
        [
            Kept::Retired("住在上海".into(), Retire::Outdated, "m1".into()),
            Kept::Revised("搬到杭州了".into()),
            Kept::Summary("用户住杭州。".into()),
        ]
    );
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
    d.apply(merging(3, 3, 3, &["m1", "m2", "m3"]), now);
    d.apply(texts(&[("m1", "黑咖啡"), ("m3", "猫咖")]), now);
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
    d.apply(merging(2, 2, 2, &["m1", "m2"]), now);
    d.apply(texts(&[("m1", "一"), ("m2", "二")]), now);
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
            Line::Retired("二".into(), Retire::Repeat, "m2".into())
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

#[test]
fn floating_waits_for_the_texts_that_are_still_on_the_way() {
    let (mut d, now) = dream();
    let every = Duration::from_millis(500);
    d.apply(merging(3, 3, 3, &["m1", "m2"]), now);
    d.tick(now + every * 4, every);
    assert_eq!(d.rows().count(), 0, "正文没回来，先不浮也不跳过");
    d.apply(texts(&[("m1", "黑咖啡"), ("m2", "猫咖")]), now + every * 4);
    d.tick(now + every * 4, every);
    assert_eq!(
        d.rows().cloned().collect::<Vec<_>>(),
        [Line::Float("黑咖啡".into())]
    );
    // 下一批开始：上一批取回的正文不算这一批的，照样等；这一批的正文到了再浮。
    d.apply(merging(6, 3, 0, &["m9"]), now + every * 5);
    d.tick(now + every * 9, every);
    assert_eq!(d.rows().count(), 1);
    d.apply(texts(&[("m9", "早上不吃早饭")]), now + every * 9);
    d.tick(now + every * 9, every);
    assert_eq!(d.rows().last(), Some(&Line::Float("早上不吃早饭".into())));
}

#[test]
fn a_retired_line_and_record_get_their_text_when_the_texts_come_late() {
    let (mut d, now) = dream();
    d.apply(merging(3, 3, 3, &["m1"]), now);
    d.apply(
        Change::Retired {
            id: "m1".into(),
            why: Retire::Outdated,
        },
        now,
    );
    d.apply(
        Change::Landed(Landed::Retired("m1".into(), Retire::Outdated)),
        now,
    );
    d.apply(texts(&[("m1", "住在上海")]), now);
    let line = Line::Retired("住在上海".into(), Retire::Outdated, "m1".into());
    assert_eq!(d.rows().last(), Some(&line));
    assert_eq!(
        d.kept(),
        [Kept::Retired(
            "住在上海".into(),
            Retire::Outdated,
            "m1".into()
        )]
    );
}

#[test]
fn the_block_shows_in_its_own_session_until_she_is_woken() {
    let now = Instant::now();
    let mut home = Dream::new(String::new(), now);
    assert!(home.shown_in(None), "首页打的、编号还没到的照画");
    home.session = "s1".into();
    assert!(home.shown_in(Some("s1")));
    assert!(!home.shown_in(Some("s2")), "看着别的会话不画");
    home.awake = true;
    assert!(!home.shown_in(Some("s1")), "叫醒了转到后台不画");
}

#[test]
fn only_a_bare_dream_starts_one_and_dream_stop_stops() {
    assert!(asks_to_dream("/dream"));
    assert!(asks_to_dream("  /dream "));
    assert!(!asks_to_dream("/dream stop"), "叫停不是又一趟");
    assert!(!asks_to_dream("/dreamy"));
    assert!(asks_to_stop("/dream stop"));
    assert!(asks_to_stop(" /dream   stop "));
    assert!(!asks_to_stop("/dream"));
    assert!(!asks_to_stop("/dream stop now"));
    assert!(!asks_to_stop("/stop"));
}
