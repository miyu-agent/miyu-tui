use super::*;

#[test]
fn the_last_whole_progress_line_of_a_piece_wins() {
    let mut lines = Lines::default();
    let got = lines.feed(
        "c1",
        "start\nmiyu-progress: 下好 1/3\nmiyu-progress:  下好 2/3 \nbye\n",
    );
    assert_eq!(got.as_deref(), Some("下好 2/3"));
}

#[test]
fn a_half_line_waits_for_the_next_piece() {
    let mut lines = Lines::default();
    assert_eq!(lines.feed("c1", "miyu-prog"), None);
    assert_eq!(lines.feed("c1", "ress: 一半"), None);
    assert_eq!(lines.feed("c1", "了\n").as_deref(), Some("一半了"));
}

#[test]
fn calls_keep_their_own_half_lines() {
    let mut lines = Lines::default();
    assert_eq!(lines.feed("a", "miyu-progress: 甲"), None);
    assert_eq!(
        lines.feed("b", "miyu-progress: 乙\n").as_deref(),
        Some("乙")
    );
    assert_eq!(lines.feed("a", "\n").as_deref(), Some("甲"));
}

#[test]
fn lines_that_only_look_alike_are_not_progress() {
    let mut lines = Lines::default();
    assert_eq!(lines.feed("c1", "Miyu-progress: 大写\n"), None);
    assert_eq!(lines.feed("c1", "miyu-progress:没空格\n"), None);
    assert_eq!(lines.feed("c1", "  miyu-progress: 前面有空格\n"), None);
    assert_eq!(lines.feed("c1", "miyu-progress:    \n"), None);
    // 不是的那一行丢到换行，下一行照常认。
    assert_eq!(
        lines
            .feed("c1", "x miyu-progress: 中间\nmiyu-progress: 好\n")
            .as_deref(),
        Some("好")
    );
}

#[test]
fn a_long_line_is_cut_and_memory_stays_bounded() {
    let mut lines = Lines::default();
    let long = "字".repeat(5000);
    assert_eq!(lines.feed("c1", &format!("miyu-progress: {long}")), None);
    assert!(lines.calls["c1"].half.len() <= HELD);
    let got = lines.feed("c1", "\n").expect("一行");
    assert_eq!(got.chars().count(), LONGEST);
}

#[test]
fn a_result_drops_what_was_held() {
    let mut lines = Lines::default();
    lines.feed("c1", "miyu-progress: 半");
    lines.over("c1");
    assert_eq!(lines.feed("c1", "行\n"), None);
}
