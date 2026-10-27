//! 回答了提问（蓝图 `tui.md`「确认和提问的抽屉」第 6 条，2026-10-26 项目主人定）：不进时间线，照你说的话的样子画——左边
//! 一根暗色竖线、上下各一行空竖线（同别处来的话）；第一行暗色「你回答了 N 个问题」，下面一题一行「问题 → 答案」照常的
//! 颜色，补充的话接在后面。

use ratatui::style::Style;
use ratatui::text::Span;

use super::rows::{Ctx, Row};
use crate::input::wrap_words;
use crate::theme;
use crate::transcript::Entry;

/// 排成的行：第一行是那一句「你回答了 N 个问题」，下面一道一行（`report_into` 写好的字）。
pub fn rows(entry: &Entry, ctx: &Ctx) -> Vec<Row> {
    let bar = Span::styled(ctx.config.layout.user_bar.clone(), theme::dim());
    let mut out = vec![ctx.row(bar.clone(), Vec::new())];
    for (i, line) in entry.text.trim_matches('\n').split('\n').enumerate() {
        let style = if i == 0 { theme::dim() } else { Style::new() };
        let mut first = true;
        for piece in wrap_words(line, ctx.width.max(1)) {
            let text = line[piece.start..piece.end].trim_end_matches('\n');
            let mut row = ctx.row(bar.clone(), vec![Span::styled(text.to_string(), style)]);
            row.joined = !std::mem::take(&mut first);
            out.push(row);
        }
    }
    out.push(ctx.row(bar, Vec::new()));
    out
}

#[cfg(test)]
mod tests {
    use ratatui::style::Style;

    use super::rows;
    use crate::theme;
    use crate::transcript::{Entry, Kind};
    use crate::ui::test_support::Fixture;

    #[test]
    fn answers_read_like_something_you_said_with_a_dim_head() {
        let f = Fixture::new();
        let entry = Entry::new(
            1,
            Kind::Answered,
            "你回答了 2 个问题\n接下来做什么 → 只是看看\n用哪个库 → serde（补充：要快）".into(),
        );
        let got = rows(&entry, &f.ctx());
        let text: Vec<String> = got.iter().map(|r| r.line.to_string()).collect();
        let bar = f.config.layout.user_bar.trim_end();
        assert_eq!(
            text.first().map(|t| t.trim()),
            Some(bar),
            "上面一行空竖线：{text:?}"
        );
        assert_eq!(
            text.last().map(|t| t.trim()),
            Some(bar),
            "下面一行空竖线：{text:?}"
        );
        assert!(text[1].contains("你回答了 2 个问题"), "{text:?}");
        assert!(text[2].contains("接下来做什么 → 只是看看"), "{text:?}");
        assert!(
            text[3].contains("用哪个库 → serde（补充：要快）"),
            "{text:?}"
        );
        let style = |row: usize| got[row].line.spans.last().map(|s| s.style);
        assert_eq!(style(1), Some(theme::dim()), "头一行暗色");
        assert_eq!(style(2), Some(Style::new()), "回答照常的颜色");
    }
}
