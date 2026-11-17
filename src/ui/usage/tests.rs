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
