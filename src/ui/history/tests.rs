//! 输入历史列表排成的行（蓝图 `tui.md`「输入历史列表」）：和命令列表、后台面板一个框；列表每条左边写多久以前，
//! 命令名、好几行的、粘贴块、搜到的字各有记号；下面预览选中的那一条，最下面是搜索那一行（2026-11-03 照 Claude Code 改版）。

use std::time::{Duration, SystemTime};

use ratatui::layout::Rect;
use ratatui::style::Modifier;

use super::{index_at, lines};
use crate::config::Config;
use crate::history::History;
use crate::input::{Draft, Sent};
use crate::theme;

fn sent(text: &str, ago: u64, now: SystemTime) -> Sent {
    Sent {
        draft: Draft::plain(text),
        at: now - Duration::from_secs(ago),
    }
}

fn plain(rows: &[super::Row]) -> Vec<String> {
    rows.iter()
        .map(|(_, l)| l.to_string().trim_end().to_string())
        .collect()
}

/// 上边框写的字。
fn title(chrome: &crate::ui::panel::Chrome) -> String {
    chrome.title.iter().map(|s| s.content.as_ref()).collect()
}

#[test]
fn time_on_the_left_preview_and_search_below_and_clicks_find_their_entry() {
    let _theme = theme::hold();
    let config = Config::builtin().unwrap();
    let now = SystemTime::now();
    let list = [
        sent("新的", 10, now),
        sent("中间", 300, now),
        sent("早的", 7200, now),
    ];
    let found: Vec<&Sent> = list.iter().collect();
    let (chrome, rows) = lines(&History::default(), &found, 40, &config, now, usize::MAX);
    let text = plain(&rows);
    // 2026-09-30 项目主人：三样一个框，标题嵌在上边框，按键提示嵌在下边框。
    assert_eq!(title(&chrome), "历史  3 条");
    assert_eq!(
        chrome.hint.as_deref(),
        Some(config.text.history.hints.as_str())
    );
    // 排期第 17 条：左边时间（露着的几条里最宽的那么宽）、右边一行预览。
    assert_eq!(text[0], "  2 小时前 早的", "{text:?}");
    assert_eq!(text[1], "  5 分钟前 中间");
    assert_eq!(text[2], "❯ 刚才     新的");
    assert_eq!(
        rows[2].1.style,
        theme::picked_bar(),
        "选中的铺强调色的底、字深色"
    );
    assert_eq!(rows[2].1.width(), 40);
    // 下面一条横线、选中那条的全文、一条横线、搜索那一行（没打字写「筛选历史…」）。
    let rule = |l: &str| !l.is_empty() && l.chars().all(|c| c == '─');
    assert!(rule(&text[3]) && rule(&text[5]), "{text:?}");
    assert_eq!(text[4], "  新的");
    assert!(
        text[6].ends_with(&config.text.history.placeholder),
        "{text:?}"
    );
    assert_eq!(rows.len(), 7);
    // 框里放字的那一块从第 10 行起：列表点得中那一条，预览算选中的，横线、搜索那一行点不中。
    let text_area = Rect::new(0, 10, 40, 7);
    assert_eq!(index_at(text_area, &rows, 12), Some(0));
    assert_eq!(index_at(text_area, &rows, 10), Some(2));
    assert_eq!(
        index_at(text_area, &rows, 14),
        Some(0),
        "预览算选中的那一条"
    );
    assert_eq!(index_at(text_area, &rows, 13), None);
    assert_eq!(index_at(text_area, &rows, 16), None);
    assert_eq!(index_at(text_area, &rows, 9), None, "框的上边点不中");
}

#[test]
fn commands_lines_pastes_and_hits_are_marked() {
    let _theme = theme::hold();
    let config = Config::builtin().unwrap();
    let now = SystemTime::now();
    let label = "[已粘贴 18 行]";
    let pasted = Sent {
        draft: Draft::from_pasted(
            &format!("{label} 看看报错"),
            &[(label.to_string(), "很长的报错".to_string())],
        ),
        at: now,
    };
    // 选中的是最新的那一条（「别的话」）：记号看的是没选中的几条。
    let list = [
        sent("别的话", 0, now),
        sent("/theme 换一套", 0, now),
        sent("第一行\n第二行\n第三行", 60, now),
        pasted,
    ];
    let found: Vec<&Sent> = list.iter().collect();
    let (_, rows) = lines(&History::default(), &found, 60, &config, now, usize::MAX);
    let span = |row: usize, text: &str| {
        rows[row]
            .1
            .spans
            .iter()
            .find(|s| s.content.contains(text))
            .map(|s| s.style)
            .unwrap_or_else(|| panic!("第 {row} 行没有「{text}」：{:?}", rows[row].1))
    };
    // 最新的在最底下：第 2 行是命令，第 1 行是好几行的，第 0 行是带粘贴块的。
    assert_eq!(span(2, "/theme").fg, theme::accent().fg, "命令名强调色");
    assert_eq!(span(2, "换一套").fg, theme::dim().fg, "没选中的字暗");
    let multi = rows[1].1.to_string();
    assert!(multi.contains("第一行") && multi.contains(" · +2 行") && !multi.contains("第二行"));
    assert_eq!(span(0, label), theme::chip(), "粘贴块照输入框里的样子");
    // 搜的字标出来。
    let history = History {
        query: "read".into(),
        ..History::default()
    };
    let list = [sent("别的", 0, now), sent("看看 README 再说", 0, now)];
    let found: Vec<&Sent> = list.iter().collect();
    let (chrome, rows) = lines(&history, &found, 60, &config, now, usize::MAX);
    assert_eq!(title(&chrome), "历史  2 条", "框线上不写搜的字");
    assert!(
        plain(&rows).last().unwrap().contains("read"),
        "搜的字在最下面那一行"
    );
    let hit = rows[0]
        .1
        .spans
        .iter()
        .find(|s| s.content == "READ")
        .map(|s| s.style)
        .unwrap();
    assert!(hit.add_modifier.contains(Modifier::UNDERLINED));
    assert_eq!(hit.fg, theme::accent().fg);
}

#[test]
fn a_long_entry_is_clipped_in_the_list_and_shown_whole_below() {
    let config = Config::builtin().unwrap();
    let now = SystemTime::now();
    let long: String = "很长的一句话".repeat(10);
    let list = [sent(&long, 0, now)];
    let found: Vec<&Sent> = list.iter().collect();
    let (_, rows) = lines(&History::default(), &found, 30, &config, now, usize::MAX);
    let text = plain(&rows);
    assert!(
        text[0].starts_with("❯ 刚才 很长") && text[0].ends_with('…'),
        "{text:?}"
    );
    assert_eq!(rows[0].1.width(), 30);
    let shown: String = text[2..text.len() - 2].iter().map(|l| l.trim()).collect();
    assert_eq!(shown, long, "预览折行写全：{text:?}");
}

#[test]
fn nothing_found_says_so_and_keeps_what_was_typed() {
    let config = Config::builtin().unwrap();
    let history = History {
        query: "xyz".into(),
        ..History::default()
    };
    let (chrome, rows) = lines(&history, &[], 40, &config, SystemTime::now(), usize::MAX);
    assert_eq!(title(&chrome), "历史  0 条");
    let text = plain(&rows);
    assert_eq!(text[0], format!("  {}", config.text.history.empty));
    assert_eq!(rows.len(), 3, "没有预览那一段：{text:?}");
    assert!(text[2].contains("xyz"));
}

#[test]
fn the_preview_keeps_its_height_and_caps_long_ones() {
    let config = Config::builtin().unwrap();
    let now = SystemTime::now();
    // 选中一行的那条时，预览照露着的里最长的（三行那条）高：上下选时框不一伸一缩。
    let list = [sent("一行", 0, now), sent("甲\n乙\n丙", 60, now)];
    let found: Vec<&Sent> = list.iter().collect();
    let mut history = History::default();
    let (_, short) = lines(&history, &found, 40, &config, now, usize::MAX);
    history.older(2);
    let (_, tall) = lines(&history, &found, 40, &config, now, usize::MAX);
    assert_eq!(short.len(), tall.len(), "{:?}", plain(&short));
    let text = plain(&tall);
    assert_eq!(text[3..6], ["  甲", "  乙", "  丙"], "{text:?}");
    // 太长的：最多 history_preview_rows 行，最后一行写还有几行。
    let long: String = (1..=20).map(|n| format!("第 {n} 行\n")).collect();
    let list = [sent(&long, 0, now)];
    let found: Vec<&Sent> = list.iter().collect();
    let (_, rows) = lines(&History::default(), &found, 40, &config, now, usize::MAX);
    let cap = config.layout.history_preview_rows;
    let text = plain(&rows);
    assert_eq!(rows.len(), 1 + 1 + cap + 1 + 1);
    assert_eq!(text[1 + cap], format!("  ⋮ 还有 {} 行", 20 - (cap - 1)));
}

#[test]
fn a_short_window_shrinks_the_preview_first_and_keeps_the_search_row() {
    let config = Config::builtin().unwrap();
    let now = SystemTime::now();
    let long: String = (1..=10).map(|n| format!("第 {n} 行\n")).collect();
    let list = [
        sent(&long, 0, now),
        sent("乙", 60, now),
        sent("丙", 120, now),
        sent("丁", 180, now),
        sent("戊", 240, now),
    ];
    let found: Vec<&Sent> = list.iter().collect();
    // 9 行：搜索那一行和横线 2 行，列表至少 3 条，剩下 4 行给预览（横线 + 3 行）。
    let (_, rows) = lines(&History::default(), &found, 40, &config, now, 9);
    let text = plain(&rows);
    assert_eq!(rows.len(), 9, "{text:?}");
    assert!(text[2].starts_with("❯ "), "列表留三条、选中的在：{text:?}");
    assert!(
        text[4].contains("第 1 行") && text[6].contains("还有"),
        "{text:?}"
    );
    assert!(text[8].ends_with(&config.text.history.placeholder));
    // 再矮到放不下预览：只剩列表和搜索那一行。
    let (_, rows) = lines(&History::default(), &found, 40, &config, now, 6);
    let text = plain(&rows);
    assert_eq!(rows.len(), 6, "{text:?}");
    assert!(
        text[3].starts_with("❯ ") && text[5].ends_with(&config.text.history.placeholder),
        "{text:?}"
    );
}
