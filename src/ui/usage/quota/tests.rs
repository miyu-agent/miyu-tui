//! 套餐额度那几行怎么写（「配置与模型」第 5 条，2026-11-14 项目主人定的样子）。

use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::{Glyphs, lines};
use crate::app::quota::QuotaWindow;
use crate::config::Config;

fn window(label: &str, percent: f64, resets_at: Option<&str>) -> QuotaWindow {
    QuotaWindow {
        label: label.into(),
        percent,
        resets_at: resets_at.map(|t| t.parse().unwrap()),
    }
}

fn text(lines: &[ratatui::text::Line]) -> Vec<String> {
    lines.iter().map(ToString::to_string).collect()
}

/// 默认主题的记号（`marks.block`、`marks.edge`）。
fn plain() -> [String; 2] {
    ["▕".to_string(), "▏".to_string()]
}

fn now() -> Timestamp {
    "2026-11-15T01:00:00Z".parse().unwrap()
}

#[test]
fn one_row_per_window_with_a_framed_bar_percent_and_reset() {
    let config = Config::builtin().unwrap();
    let texts = &config.text.usage;
    let windows = [
        window("5 小时", 4.0, Some("2026-11-15T03:00:00Z")),
        window("每周", 50.4, Some("2026-11-20T00:00:00Z")),
    ];
    let got = text(&lines(
        texts,
        &windows,
        (now(), &TimeZone::UTC),
        80,
        &Glyphs {
            block: "█",
            edge: &plain(),
        },
    ));
    assert_eq!(
        got[0],
        format!("5 小时  ▕█{}▏    4%  2 小时后重置", " ".repeat(19)),
        "主人给的样子：名字、带框的条、百分比、重置时刻"
    );
    assert_eq!(
        got[1],
        format!(
            "每周    ▕{}{}▏   50%  11-20 00:00 重置",
            "█".repeat(10),
            " ".repeat(10)
        ),
        "名字对齐，24 小时以外写日期时刻"
    );
}

#[test]
fn no_quota_draws_nothing_and_soon_is_minutes() {
    let config = Config::builtin().unwrap();
    let texts = &config.text.usage;
    assert!(
        lines(
            texts,
            &[],
            (now(), &TimeZone::UTC),
            80,
            &Glyphs {
                block: "█",
                edge: &plain()
            }
        )
        .is_empty()
    );
    let soon = [window("5 小时", 0.0, Some("2026-11-15T01:30:00Z"))];
    let got = text(&lines(
        texts,
        &soon,
        (now(), &TimeZone::UTC),
        80,
        &Glyphs {
            block: "█",
            edge: &plain(),
        },
    ));
    assert!(got[0].ends_with("30 分钟后重置"), "{}", got[0]);
    assert!(
        got[0].contains(&format!("▕{}▏", " ".repeat(20))),
        "没用的条是空的"
    );
}

#[test]
fn eighty_percent_turns_yellow_and_full_turns_red() {
    let config = Config::builtin().unwrap();
    let texts = &config.text.usage;
    let windows = [
        window("a", 79.0, None),
        window("b", 80.0, None),
        window("c", 100.0, None),
    ];
    let got = lines(
        texts,
        &windows,
        (now(), &TimeZone::UTC),
        80,
        &Glyphs {
            block: "█",
            edge: &plain(),
        },
    );
    let fg = |line: &ratatui::text::Line| line.spans[2].style.fg;
    assert_eq!(fg(&got[0]), None, "八成以下照平常的字");
    assert_eq!(fg(&got[1]), crate::theme::warn().fg);
    assert_eq!(fg(&got[2]), crate::theme::error().fg);
    assert_ne!(crate::theme::warn().fg, crate::theme::error().fg);
}

#[test]
fn ascii_theme_uses_brackets_and_narrow_drops_the_reset() {
    let config = Config::builtin().unwrap();
    let texts = &config.text.usage;
    let windows = [window("5 小时", 50.0, Some("2026-11-15T03:00:00Z"))];
    let got = text(&lines(
        texts,
        &windows,
        (now(), &TimeZone::UTC),
        80,
        &Glyphs {
            block: "#",
            edge: &["[".to_string(), "]".to_string()],
        },
    ));
    assert!(
        got[0].contains(&format!("[{}{}]", "#".repeat(10), " ".repeat(10))),
        "{}",
        got[0]
    );
    let narrow = text(&lines(
        texts,
        &windows,
        (now(), &TimeZone::UTC),
        24,
        &Glyphs {
            block: "█",
            edge: &plain(),
        },
    ));
    assert!(
        !narrow[0].contains("重置"),
        "放不下不写重置时刻：{}",
        narrow[0]
    );
    assert!(narrow[0].width_ok(24));
}

trait WidthOk {
    fn width_ok(&self, width: usize) -> bool;
}

impl WidthOk for String {
    fn width_ok(&self, width: usize) -> bool {
        unicode_width::UnicodeWidthStr::width(self.as_str()) <= width
    }
}

#[test]
fn rows_line_up_as_a_grid_when_the_reset_texts_differ_in_width() {
    // 2026-11-17 验收第 36 条：「1 小时后重置」和「11-23 09:00 重置」宽窄不同，窄的时候条也要一样长、各列对齐。
    let config = Config::builtin().unwrap();
    let texts = &config.text.usage;
    let windows = [
        window("5 小时", 0.0, Some("2026-11-15T02:00:00Z")),
        window("每周", 1.0, Some("2026-11-23T09:00:00Z")),
        window("每月", 42.0, None),
    ];
    for width in [80, 40, 36] {
        let got = lines(
            texts,
            &windows,
            (now(), &TimeZone::UTC),
            width,
            &Glyphs {
                block: "█",
                edge: &plain(),
            },
        );
        let bar = |line: &ratatui::text::Line| line.spans[2].width();
        let percent_at =
            |line: &ratatui::text::Line| line.spans[..4].iter().map(|s| s.width()).sum::<usize>();
        assert_eq!(bar(&got[0]), bar(&got[1]), "条一样长（宽 {width}）");
        assert_eq!(bar(&got[1]), bar(&got[2]), "条一样长（宽 {width}）");
        assert_eq!(
            percent_at(&got[0]),
            percent_at(&got[1]),
            "百分比一列（宽 {width}）"
        );
        let with_reset: Vec<usize> = got.iter().map(|l| l.spans.len()).collect();
        assert!(
            with_reset[..2] == [6, 6] || with_reset[..2] == [5, 5],
            "重置时刻要么都写、要么都不写（宽 {width}）：{with_reset:?}"
        );
        assert!(
            got.iter().all(|l| l.width() <= usize::from(width)),
            "放得下（宽 {width}）"
        );
    }
}
