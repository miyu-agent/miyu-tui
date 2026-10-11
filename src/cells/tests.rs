use unicode_width::UnicodeWidthStr;

use super::{clip, tail, tail_marked, wrap};

/// `⚠️` 是两个字符、一个字素簇，两列宽。
const WARN: &str = "⚠️";

#[test]
fn clipping_counts_a_warning_sign_as_two_columns() {
    let text = format!("{WARN}{WARN}{WARN}abc");
    let cut = clip(&text, 5);
    assert_eq!(cut, format!("{WARN}{WARN}…"));
    assert!(cut.width() <= 5);
    assert_eq!(clip("abc", 3), "abc");
}

#[test]
fn tails_keep_the_end_within_the_width() {
    let text = format!("abc{WARN}{WARN}{WARN}");
    assert_eq!(tail(&text, 5), format!("{WARN}{WARN}"));
    let marked = tail_marked(&text, 5);
    assert_eq!(marked, format!("…{WARN}{WARN}"));
    assert!(marked.width() <= 5);
    assert_eq!(tail_marked("abc", 3), "abc");
}

#[test]
fn wrapping_never_splits_or_overfills_a_cluster() {
    let rows = wrap(&format!("{WARN}{WARN}{WARN}"), 4);
    assert_eq!(rows, [format!("{WARN}{WARN}"), WARN.to_string()]);
    assert_eq!(wrap("", 4), [""]);
    assert_eq!(wrap("中文", 1), ["中", "文"], "放不下一个字的也独占一行");
}
