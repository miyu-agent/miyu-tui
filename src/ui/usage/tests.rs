//! `/usage` 一页怎么排（2026-11-17 验收）：按会话的列表很长时框不占大半屏（第 5 条），套餐额度上面有标题（第 6 条）。

use ratatui::text::Line;

use super::{Sources, page_lines};
use crate::app::UsageData;
use crate::app::quota::QuotaWindow;
use crate::config::Config;
use crate::core::UsageRow;

/// 一天用过、`sessions` 个会话各用过一些。
fn data(sessions: usize) -> UsageData {
    let today = jiff::civil::date(2026, 11, 17);
    let day = UsageRow {
        day: Some("2026-11-17".into()),
        requests: 1,
        tokens: 110,
        ..UsageRow::default()
    };
    let rows = (0..sessions)
        .map(|i| UsageRow {
            session: Some(format!("0000000{i:04}")),
            requests: 1,
            tokens: 1000 - i as u64,
            ..UsageRow::default()
        })
        .collect();
    UsageData {
        total: Some(Ok(vec![day.clone()])),
        days: Some(Ok(vec![day])),
        models: Some(Ok(Vec::new())),
        sessions: Some(Ok(rows)),
        purposes: Some(Ok(Vec::new())),
        today,
        rates: Vec::new(),
        quota: Vec::new(),
        balance: None,
        quota_from: None,
    }
}

fn text(lines: &[Line]) -> Vec<String> {
    lines.iter().map(ToString::to_string).collect()
}

/// 排第 `tab` 页，滚到 `scroll`，框里最多放 `max` 行。
fn page(
    config: &Config,
    data: &UsageData,
    (tab, scroll): (usize, usize),
    max: usize,
) -> Vec<String> {
    let src = Sources {
        config,
        currency: "USD",
        sessions: &[],
        provider: "OpenCode Go".into(),
    };
    text(&page_lines(&src, data, (tab, scroll, false), 120, max).1)
}

#[test]
fn a_long_session_list_does_not_make_the_box_take_most_of_the_screen() {
    let config = Config::builtin().unwrap();
    let data = data(100);
    let overview = page(&config, &data, (0, 0), 50);
    assert!(
        overview.len() <= 2 + 24,
        "总览不比放得下的一半高：{} 行",
        overview.len()
    );
    let sessions = page(&config, &data, (3, 0), 50);
    assert_eq!(sessions.len(), overview.len(), "换页不改框的高度");
    let last = sessions.last().unwrap();
    assert!(
        last.starts_with("⋮ 还有 ") && last.ends_with(" 行"),
        "{last}"
    );
    // 滚到底：最后一个会话露出来，不再写还有几行。
    let end = page(&config, &data, (3, 1000), 50);
    assert!(end.last().unwrap().contains("#00000099"), "{end:?}");
    assert!(!end.iter().any(|l| l.starts_with("⋮ 还有 ")));
}

#[test]
fn short_pages_keep_the_tallest_height() {
    // 会话少、放得下的：照四页里最高的那一页，不写「还有」（2026-10-07 项目主人：换页不改框的高度）。
    let config = Config::builtin().unwrap();
    let data = data(3);
    let sessions = page(&config, &data, (3, 0), 50);
    assert_eq!(page(&config, &data, (0, 0), 50).len(), sessions.len());
    assert!(!sessions.iter().any(|l| l.starts_with("⋮ 还有 ")));
}

#[test]
fn the_quota_windows_have_a_title_above_them() {
    let config = Config::builtin().unwrap();
    let mut data = data(1);
    let without = page(&config, &data, (0, 0), 50);
    assert!(!without.iter().any(|l| l.contains("额度情况")));
    data.quota = vec![QuotaWindow {
        label: "5 小时".into(),
        percent: 4.0,
        resets_at: None,
    }];
    let with = page(&config, &data, (0, 0), 50);
    assert_eq!(with[2], "OpenCode Go 额度情况", "照供应商的名字：{with:?}");
    assert!(with[3].starts_with("5 小时"), "{with:?}");
}
