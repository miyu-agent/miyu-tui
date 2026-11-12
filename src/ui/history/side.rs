//! 宽窗口的输入历史列表：左边列表、右边预览，一样高，中间一条暗色竖线（蓝图 `tui.md`「输入历史列表」第 1 条「宽窗口预览
//! 放右边」，2026-11-12 项目主人）。竖线右边空一格写预览，不套小框；上边框、横线上接竖线的那两格由 `draw` 补。

use ratatui::text::{Line, Span};

use super::Row;
use super::preview;
use crate::config::HistoryTexts;
use crate::input::Sent;
use crate::theme;

/// 左边排好的列表（`left` 列宽、已经垫满 `rows` 行，最新的贴着底）接上右边 `cols` 列的预览，一行一行并起来。行算左边那一条：
/// 点右边的预览算选中的那一条，由 `index_at` 照 `Side` 认。
pub fn join(left: Vec<Row>, sent: Option<&Sent>, cols: u16, words: &HistoryTexts) -> Vec<Row> {
    let body = preview::body(sent, cols, left.len(), words);
    let bar = crate::sheet::read(|s| s.frame.boxes.lines()).vertical;
    left.into_iter()
        .zip(body)
        .map(|((item, line), right)| {
            // 选中的那一条每一段都铺了底（`panel::bar`）：整行的底不留，免得铺到右边。
            let mut spans = line.spans;
            spans.push(Span::styled(format!("{bar} "), theme::dim()));
            spans.extend(right);
            (item, Line::from(spans))
        })
        .collect()
}
