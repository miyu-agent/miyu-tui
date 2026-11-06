//! 前缀要变时问人的框（蓝图「前缀要变」第 3 条）：盖在输入框上面，标题「下一轮的缓存会失效」，「这些文件改了：」下面一个
//! 文件一行（暗色、照原样），多的写「还有 N 个」；空一行，一排三个按钮，停着的铺选中色（同软件包页卸载弹窗的按钮）。

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::panel::{self, Chrome};
use crate::app::prefix_ask::PrefixAsk;
use crate::config::PrefixTexts;
use crate::theme;

/// 框的标题和一行行（交给 `panels.rs`）；按钮那一行在映射里是 `Some(0)`，点击照横坐标认哪个（`app/prefix_ask.rs`）。
pub fn lines(
    ask: Option<&PrefixAsk>,
    button: usize,
    words: &PrefixTexts,
    width: u16,
    max: usize,
) -> (Chrome, Vec<Line<'static>>, Vec<Option<usize>>) {
    let chrome = Chrome::new(&words.title, Vec::new());
    let plain = |text: String, style: Style| {
        panel::item(false, vec![Span::styled(text, style)], None, width)
    };
    let mut rows: Vec<panel::Row> = vec![(None, plain(words.files.clone(), Style::new()))];
    // 标题行、空行、按钮那一行一直在；文件放得下几行放几行，放不下的连同核心没列出来的写成「还有 N 个」。
    let (files, more) = ask.map_or((&[][..], 0), |a| (a.files.as_slice(), a.more));
    let room = max.saturating_sub(3);
    let total = files.len() as u64 + more;
    let shown = if total as usize <= room {
        files.len()
    } else {
        room.saturating_sub(1).min(files.len())
    };
    for file in &files[..shown] {
        rows.push((None, plain(format!("  {file}"), theme::dim())));
    }
    let left = total - shown as u64;
    if left > 0 && room > shown {
        let more = words.more.replace("{n}", &left.to_string());
        rows.push((None, plain(format!("  {more}"), theme::dim())));
    }
    rows.push((None, plain(String::new(), Style::new())));
    let mut spans = Vec::new();
    for (i, label) in words.buttons.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let style = if i == button {
            theme::picked_bar()
        } else {
            Style::new()
        };
        spans.push(Span::styled(format!("[ {label} ]"), style));
    }
    rows.push((Some(0), panel::item(false, spans, None, width)));
    // 窗口矮得连这几行都放不下的，留最下面的（按钮那一行）。
    let rows = rows.split_off(rows.len().saturating_sub(max));
    let (map, lines) = rows.into_iter().unzip();
    (chrome, lines, map)
}

#[cfg(test)]
mod tests;
