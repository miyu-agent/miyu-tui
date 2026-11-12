//! 输入历史列表的预览：选中那一条的全文（原来的换行照留，太长的折行），固定几行，短的下面空着，再长的最后一行写还有几行
//! （蓝图 `tui.md`「输入历史列表」第 1 条）。上下排时装在一个圆角框里（发版前 check 第 31 条），预览在右边时不套框（第 52 条）。

use ratatui::text::{Line, Span};

use super::super::panel;
use crate::config::HistoryTexts;
use crate::input::{Sent, pieces};
use crate::theme;

/// 上下排时预览框里能写几列：左右的边各占一格、再各空一格，字和列表的字对齐。
pub fn inner(width: u16) -> u16 {
    width.saturating_sub(4).max(1)
}

/// 一条的全文折成的几行：原来的换行照留，太长的折行。
pub fn wrapped(sent: &Sent, cols: u16) -> Vec<String> {
    pieces(sent.draft.text.trim_end(), cols)
        .into_iter()
        .map(|(piece, _)| piece)
        .collect()
}

/// 预览的字：`height` 行，每行 `cols` 列以内；短的补空行，再长的最后一行写还有几行。一条都对不上（`sent` 是 `None`）的
/// 全是空行（框照样占着位，第 52 条）。
pub fn body(
    sent: Option<&Sent>,
    cols: u16,
    height: usize,
    words: &HistoryTexts,
) -> Vec<Vec<Span<'static>>> {
    let all = sent.map(|s| wrapped(s, cols)).unwrap_or_default();
    let shown = if all.len() > height {
        height.saturating_sub(1)
    } else {
        all.len()
    };
    let mut body: Vec<Vec<Span<'static>>> = all[..shown]
        .iter()
        .map(|piece| vec![Span::raw(piece.clone())])
        .collect();
    if all.len() > shown && height > 0 {
        let note = words
            .more
            .replace("{count}", &(all.len() - shown).to_string());
        body.push(vec![Span::styled(note, theme::faint())]);
    }
    body.resize(height, Vec::new());
    body.into_iter()
        .map(|spans| panel::clip_spans(spans, usize::from(cols)))
        .collect()
}

/// 上下排的预览：`height` 行字装在一个圆角框里（框线照主题的 `frame.boxes`，同外面的框；发版前 check 第 31 条，2026-11-09
/// 项目主人：原来上下两条横线隔开「有点丑」）。
pub fn boxed(
    sent: Option<&Sent>,
    width: u16,
    height: usize,
    words: &HistoryTexts,
) -> Vec<Line<'static>> {
    let set = crate::sheet::read(|s| s.frame.boxes.set());
    let across = set
        .horizontal_top
        .repeat(usize::from(width.saturating_sub(2)));
    let edge =
        |left: &str, right: &str| Line::styled(format!("{left}{across}{right}"), theme::dim());
    let cols = usize::from(inner(width));
    let mut out = vec![edge(set.top_left, set.top_right)];
    out.extend(
        body(sent, inner(width), height, words)
            .into_iter()
            .map(|spans| {
                let used: usize = spans.iter().map(Span::width).sum();
                let mut line = vec![Span::styled(
                    format!("{} ", set.vertical_left),
                    theme::dim(),
                )];
                line.extend(spans);
                line.push(Span::raw(" ".repeat(cols.saturating_sub(used))));
                line.push(Span::styled(
                    format!(" {}", set.vertical_right),
                    theme::dim(),
                ));
                Line::from(line)
            }),
    );
    out.push(edge(set.bottom_left, set.bottom_right));
    out
}

/// 隔开列表、预览和搜索那一行的暗色横线（照主题的框线）。
pub fn rule(width: u16) -> Line<'static> {
    let line = crate::sheet::read(|s| s.frame.boxes.set()).horizontal_top;
    Line::styled(line.repeat(usize::from(width)), theme::dim())
}
