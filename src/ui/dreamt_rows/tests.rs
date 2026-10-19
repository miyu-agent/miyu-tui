use ratatui::style::Modifier;

use super::rows;
use crate::dream::{Kept, Retire};
use crate::theme;
use crate::transcript::{Kind, Transcript};
use crate::ui::rows::{Row, Target};
use crate::ui::test_support::Fixture;

fn text(row: &Row) -> String {
    row.line.spans.iter().map(|s| s.content.as_ref()).collect()
}

fn receipt(kept: Vec<Kept>) -> Transcript {
    let mut t = Transcript::default();
    t.note(
        Kind::Dreamt,
        "整理完成：检查 9 条，修改 1 条，作废 1 条，摘要已更新".into(),
    );
    t.entries[0].kept = kept;
    t
}

#[test]
fn the_receipt_is_one_dim_line_with_a_dim_star_and_nothing_to_open_without_changes() {
    let f = Fixture::new();
    let t = receipt(Vec::new());
    let got = rows(0, &t.entries[0], &f.ctx());
    assert_eq!(got.len(), 1);
    assert_eq!(
        text(&got[0]),
        "  ✦ 整理完成：检查 9 条，修改 1 条，作废 1 条，摘要已更新"
    );
    assert!(
        got[0]
            .line
            .spans
            .iter()
            .rev()
            .take(2)
            .all(|s| s.style == theme::dim()),
        "整行暗色"
    );
    assert_eq!(got[0].target, None, "没有改、删、摘要的点不开");
}

#[test]
fn opening_lists_what_was_kept_in_the_dream_style_and_wraps_long_text() {
    let f = Fixture::new();
    let mut t = receipt(vec![
        Kept::Retired("住在上海".into(), Retire::Outdated),
        Kept::Revised("搬到杭州了".into()),
        Kept::Summary("用户住在杭州，喜欢黑咖啡。".repeat(8)),
    ]);
    let closed = rows(0, &t.entries[0], &f.ctx());
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].target, Some(Target::Entry(0)), "有东西的能点开");
    t.entries[0].open = true;
    let got = rows(0, &t.entries[0], &f.ctx());
    let lines: Vec<String> = got.iter().map(text).collect();
    assert!(got.iter().all(|r| r.shade), "点开整块铺底色");
    assert_eq!(lines[1].trim(), "");
    assert_eq!(lines[2], "    ✗ 住在上海 · 过时");
    assert_eq!(lines[3], "    ✎ 搬到杭州了");
    assert!(
        lines[4].starts_with("    ✦ 摘要：用户住在杭州"),
        "{}",
        lines[4]
    );
    assert!(
        lines[5].starts_with("      "),
        "摘要长了折行，和字对齐：{}",
        lines[5]
    );
    assert_eq!(lines.last().map(|l| l.trim()), Some(""));
    let span = |row: usize, text: &str| {
        got[row]
            .line
            .spans
            .iter()
            .find(|s| s.content.contains(text))
            .unwrap()
            .style
    };
    assert_eq!(span(2, "✗"), theme::removed());
    assert!(
        span(2, "住在上海")
            .add_modifier
            .contains(Modifier::CROSSED_OUT)
    );
    assert_eq!(span(3, "✎"), theme::accent());
    assert_eq!(span(4, "✦"), theme::good());
}
