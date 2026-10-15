//! 别处来的话（蓝图 `tui.md`「别处来的话」，2026-09-30 项目主人定）：和你说的话一样左边一根竖线、上下各一行空竖线，竖线和字
//! 都暗；收着是一行 `从 B 收到消息 · 预览`（换行换成空格，放不下截掉），点一下这一行换成来处、空一行、全文，再点收起。
//! 附的图片、文件（核心 C-9）在全文里照你说的话里的附件块写，点块用系统的程序打开那个文件。

use ratatui::style::Style;
use ratatui::text::Span;

use super::rows::{Ctx, Row, Target, clip};
use crate::input::wrap_words;
use crate::theme;
use crate::transcript::Entry;

/// 排成的行。`i` 是它在正文里是第几条，`from` 是来处那一句。
pub fn rows(i: usize, entry: &Entry, from: &str, ctx: &Ctx) -> Vec<Row> {
    let target = Target::Entry(i);
    let style = if ctx.hover == Some(target) {
        theme::hover()
    } else {
        theme::dim()
    };
    let bar = Span::styled(ctx.config.layout.user_bar.clone(), theme::dim());
    let said = entry.text.trim_matches('\n');
    let mut out = vec![ctx.row(bar.clone(), Vec::new())];
    if entry.open {
        let mut head = ctx.row(bar.clone(), vec![Span::styled(from.to_string(), style)]);
        head.copy = false;
        out.push(head);
        // 来处和全文之间空一行（2026-09-30 项目主人）。
        out.push(ctx.row(bar.clone(), Vec::new()));
        let mut prev_end = None;
        let (text, pieces) = super::user_rows::shaped(said, &entry.pasted, false, false);
        let styled: Vec<_> = pieces.iter().map(|p| (p.from, p.to, p.style)).collect();
        for line in wrap_words(&text, ctx.width.max(1)) {
            let end = line.start + text[line.start..line.end].trim_end_matches('\n').len();
            // 块照块的样子，别的字暗（悬停亮一档）。
            let spans = super::user_rows::spans(&text, line.start, end, &styled)
                .into_iter()
                .map(|s| {
                    if s.style == Style::new() {
                        s.style(style)
                    } else {
                        s
                    }
                })
                .collect();
            let mut row = ctx.row(bar.clone(), spans);
            row.links = super::user_rows::links(&text, line.start, end, &pieces);
            row.joined = prev_end == Some(line.start);
            prev_end = Some(line.end);
            out.push(row);
        }
    } else {
        let flat = said.split_whitespace().collect::<Vec<_>>().join(" ");
        let words = &ctx.config.text.jobs;
        let line = words
            .received
            .replace("{from}", from)
            .replace("{text}", &flat);
        let mut row = ctx.row(
            bar.clone(),
            vec![Span::styled(clip(&line, ctx.width), style)],
        );
        row.copy = false;
        out.push(row);
    }
    out.push(ctx.row(bar, Vec::new()));
    for row in &mut out {
        row.target = Some(target);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::rows;
    use crate::theme;
    use crate::transcript::Transcript;
    use crate::ui::rows::Row;
    use crate::ui::test_support::Fixture;

    fn text(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|r| r.line.to_string().trim_end().to_string())
            .collect()
    }

    #[test]
    fn a_message_from_elsewhere_is_one_dim_line_that_opens_in_place() {
        // 2026-09-30 项目主人：别的 harness、子代理发来的画成一个样子，左边粗竖线，字都暗，`从 xx 收到消息 · 预览`，
        // 点开替换成全文。
        let f = Fixture::new();
        let mut t = Transcript::default();
        t.foreign(
            Some(3),
            "从 B 收到消息".into(),
            "我是 B：\n只回复「收到 B」".into(),
        );
        let entry = &t.entries[0];
        let closed = rows(0, entry, "从 B 收到消息", &f.ctx());
        assert_eq!(
            text(&closed),
            ["┃", "┃ 从 B 收到消息 · 我是 B： 只回复「收到 B」", "┃"]
        );
        let line = &closed[1].line.spans;
        let dim = |needle: &str| {
            line.iter()
                .any(|s| s.content.contains(needle) && s.style == theme::dim())
        };
        assert!(
            dim("┃") && dim("从 B 收到消息 · 我是 B"),
            "竖线、字都暗：{line:?}"
        );
        assert!(closed.iter().all(|r| r.target.is_some()), "整条能点");
        let mut open = entry.clone();
        open.open = true;
        let opened = rows(0, &open, "从 B 收到消息", &f.ctx());
        assert_eq!(
            text(&opened),
            [
                "┃",
                "┃ 从 B 收到消息",
                "┃",
                "┃ 我是 B：",
                "┃ 只回复「收到 B」",
                "┃"
            ]
        );
        let body = &opened[3].line.spans;
        assert!(
            body.iter()
                .any(|s| s.content.contains("我是 B") && s.style == theme::dim()),
            "全文也暗"
        );
    }

    #[test]
    fn an_attachment_from_elsewhere_is_a_chip_that_opens_its_file() {
        // 核心 C-9：别的会话发来的话带附件，点开的全文里照你说的话里的附件块写（品红字、暗紫底），点块打开那个文件。
        let f = Fixture::new();
        let mut t = Transcript::default();
        t.foreign(Some(3), "从 B 收到消息".into(), "看这张".into());
        let png = crate::core::Attached {
            media_type: "image/png".into(),
            name: Some("晚霞.png".into()),
            path: Some("/tmp/晚霞.png".into()),
        };
        t.attach_to(3, &[png], &f.config.text.attach);
        let mut open = t.entries[0].clone();
        open.open = true;
        let opened = rows(0, &open, "从 B 收到消息", &f.ctx());
        let body = opened
            .iter()
            .find(|r| r.line.to_string().contains("[图片 1]"))
            .expect("全文里有块");
        assert!(
            body.line
                .spans
                .iter()
                .any(|s| s.content.contains("[图片 1]") && s.style == theme::chip()),
            "块照块的样子：{:?}",
            body.line.spans
        );
        assert!(
            body.line
                .spans
                .iter()
                .any(|s| s.content.contains("看这张") && s.style == theme::dim()),
            "别的字照旧暗"
        );
        assert_eq!(body.links.len(), 1, "块能点");
        assert_eq!(body.links[0].2, "/tmp/晚霞.png");
    }
}
