//! 表格（蓝图 `tui.md`「她的回答：Markdown」第 8 条，规则照旧版 `render/table.rs`）：实线画满格子，
//! 每两行之间一条横线；列宽照显示宽度，放不下时收最宽的列，保底宽度照旧版；格子里的字按列宽折行。

use pulldown_cmark::Alignment;
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

use super::inline::{Folded, Piece, fold, plain};
use crate::theme;

/// 一格：片段。
pub type Cell = Vec<Piece>;

/// 画一张表，交回折好的行。`width` 是整张表最多几列宽。
pub fn render(head: &[Cell], rows: &[Vec<Cell>], aligns: &[Alignment], width: u16) -> Vec<Folded> {
    let columns = head.len().max(rows.iter().map(Vec::len).max().unwrap_or(0));
    if columns == 0 {
        return Vec::new();
    }
    let natural: Vec<usize> = (0..columns)
        .map(|c| {
            std::iter::once(head)
                .chain(rows.iter().map(Vec::as_slice))
                .filter_map(|row| row.get(c))
                .map(cell_width)
                .max()
                .unwrap_or(1)
                .max(1)
        })
        .collect();
    let widths = bounded(natural, usize::from(width));
    if widths.iter().sum::<usize>() + widths.len() * 3 + 1 > usize::from(width) {
        return stacked(head, rows, width);
    }
    // 线照主题的 `frame.table`（出厂直角）。
    let set = crate::sheet::read(|s| s.frame.table.lines());
    let rule = |left: &str, mid: &str, right: &str| {
        let body: Vec<String> = widths
            .iter()
            .map(|w| set.horizontal.repeat(w + 2))
            .collect();
        line(vec![Span::styled(
            format!("{left}{}{right}", body.join(mid)),
            theme::dim(),
        )])
    };
    let mut out = vec![rule(set.top_left, set.horizontal_down, set.top_right)];
    out.extend(row_lines(head, &widths, aligns, true));
    for row in rows {
        out.push(rule(set.vertical_right, set.cross, set.vertical_left));
        out.extend(row_lines(row, &widths, aligns, false));
    }
    out.push(rule(set.bottom_left, set.horizontal_up, set.bottom_right));
    out
}

/// 放不下时，每次把最宽的一列收窄一格，收到保底宽度为止（照旧版的 `bounded_table_widths`）。
fn bounded(mut widths: Vec<usize>, room: usize) -> Vec<usize> {
    // 每列左右各空一格、一条竖线，最右边再一条。
    let frame = widths.len() * 3 + 1;
    let floor = match widths.len() {
        1 => 16,
        2 => 14,
        3 | 4 => 10,
        _ => 8,
    };
    while widths.iter().sum::<usize>() + frame > room {
        let Some((i, w)) = widths.iter().copied().enumerate().max_by_key(|(_, w)| *w) else {
            break;
        };
        if w <= floor {
            break;
        }
        widths[i] -= 1;
    }
    widths
}

/// 收到保底宽度还放不下的表一行一段（2026-11-02 项目主人照 Claude Code 定）：一格一行「表头：值」，表头加粗、表头色，
/// 放不下的照宽折行；段与段之间一条暗色横线。
fn stacked(head: &[Cell], rows: &[Vec<Cell>], width: u16) -> Vec<Folded> {
    let set = crate::sheet::read(|s| s.frame.table.lines());
    let rule = set.horizontal.repeat(usize::from(width).min(RULE));
    let mut out = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        if r > 0 {
            out.push(line(vec![Span::styled(rule.clone(), theme::dim())]));
        }
        for (c, cell) in row.iter().enumerate() {
            let title: String = head
                .get(c)
                .map(|h| h.iter().map(|p| p.text.as_str()).collect())
                .unwrap_or_default();
            let title = title.trim();
            let mut pieces = Vec::new();
            if !title.is_empty() {
                // 表头是中日文的接全角冒号，不然接「: 」。
                let wide = title
                    .chars()
                    .last()
                    .is_some_and(|ch| ch.to_string().width() > 1);
                let colon = if wide { "：" } else { ": " };
                let style = theme::md_table_head();
                pieces.push(Piece::new(format!("{title}{colon}"), style));
            }
            pieces.extend(cell.iter().cloned());
            out.extend(fold(&pieces, width.max(1)));
        }
    }
    out
}

/// 一行一段时段与段之间那条横线最长几列。
const RULE: usize = 16;

fn cell_width(cell: &Cell) -> usize {
    let text: String = cell.iter().map(|p| p.text.as_str()).collect();
    text.split('\n')
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0)
}

/// 一行格子：每格按列宽折好，行高照最高的那格，矮的补空。
fn row_lines(row: &[Cell], widths: &[usize], aligns: &[Alignment], head: bool) -> Vec<Folded> {
    let empty = Cell::new();
    let v = crate::sheet::read(|s| s.frame.table.lines().vertical);
    let cells: Vec<Vec<Folded>> = widths
        .iter()
        .enumerate()
        .map(|(c, w)| {
            let pieces: Vec<Piece> = row
                .get(c)
                .unwrap_or(&empty)
                .iter()
                .map(|p| {
                    if head {
                        Piece {
                            style: p.style.patch(theme::md_table_head()),
                            ..p.clone()
                        }
                    } else {
                        p.clone()
                    }
                })
                .collect();
            let mut folded = fold(&pieces, u16::try_from(*w).unwrap_or(u16::MAX));
            folded.iter_mut().for_each(|f| fit(f, *w));
            folded
        })
        .collect();
    let height = cells.iter().map(Vec::len).max().unwrap_or(1);
    (0..height)
        .map(|r| {
            let mut spans = Vec::new();
            let mut links = Vec::new();
            let mut col = 0u16;
            for (c, w) in widths.iter().enumerate() {
                let bar = if c == 0 {
                    format!("{v} ")
                } else {
                    format!(" {v} ")
                };
                spans.push(Span::styled(bar.clone(), theme::dim()));
                col += u16::try_from(bar.width()).unwrap_or(0);
                let folded = cells[c].get(r);
                let used = folded.map_or(0, |f| plain(f).width());
                let gap = w.saturating_sub(used);
                let (before, after) = match aligns.get(c) {
                    Some(Alignment::Right) => (gap, 0),
                    Some(Alignment::Center) => (gap / 2, gap - gap / 2),
                    _ => (0, gap),
                };
                spans.push(Span::raw(" ".repeat(before)));
                col += u16::try_from(before).unwrap_or(0);
                if let Some(f) = folded {
                    links.extend(
                        f.links
                            .iter()
                            .map(|(a, b, u)| (a + col, b + col, u.clone())),
                    );
                    spans.extend(f.spans.iter().cloned());
                }
                spans.push(Span::raw(" ".repeat(after)));
                col += u16::try_from(used + after).unwrap_or(0);
            }
            spans.push(Span::styled(format!(" {v}"), theme::dim()));
            Folded {
                spans,
                links,
                joined: false,
            }
        })
        .collect()
}

/// 折行时满了的那一格是空白的，留在行末（`linebreak.rs`：正文里超出的看不见）；格子里它会把竖线往右顶一格（发版前
/// check 11，2026-11-08 项目主人报框线错位），超出列宽的行末空白去掉。
fn fit(folded: &mut Folded, width: usize) {
    let mut over = plain(folded).width().saturating_sub(width);
    while over > 0 {
        let Some(last) = folded.spans.last_mut() else {
            return;
        };
        let text = last.content.to_string();
        let kept = text.trim_end_matches(char::is_whitespace);
        let cut = (text.len() - kept.len()).min(over);
        if cut == 0 {
            return;
        }
        last.content = text[..text.len() - cut].to_string().into();
        over -= cut;
        if last.content.is_empty() {
            folded.spans.pop();
        }
    }
}

fn line(spans: Vec<Span<'static>>) -> Folded {
    Folded {
        spans,
        links: Vec::new(),
        joined: false,
    }
}

#[cfg(test)]
mod tests {
    use pulldown_cmark::Alignment;
    use ratatui::style::Style;
    use unicode_width::UnicodeWidthStr;

    use super::{bounded, render};
    use crate::markdown::inline::{Piece, plain};

    fn cell(text: &str) -> Vec<Piece> {
        vec![Piece::new(text, Style::new())]
    }

    #[test]
    fn a_table_has_rules_between_every_row_and_aligns_cells() {
        let head = vec![cell("左"), cell("右")];
        let rows = vec![vec![cell("a"), cell("b")], vec![cell("长一点"), cell("c")]];
        let lines: Vec<String> = render(&head, &rows, &[Alignment::Left, Alignment::Right], 40)
            .iter()
            .map(plain)
            .collect();
        assert_eq!(
            lines,
            vec![
                "┌────────┬────┐",
                "│ 左     │ 右 │",
                "├────────┼────┤",
                "│ a      │  b │",
                "├────────┼────┤",
                "│ 长一点 │  c │",
                "└────────┴────┘",
            ]
        );
        assert!(
            lines.iter().all(|l| l.width() == lines[0].width()),
            "每行一样宽"
        );
    }

    /// 发版前 check 11（2026-11-08 项目主人：表格框线有时略微错位，像是软换行导致的）：格子里折在空白上的那一行，空白
    /// 留在行末超出列宽，把后面的竖线顶出去一格。
    #[test]
    fn a_cell_folded_on_a_space_keeps_the_rules_in_line() {
        let head = vec![cell("甲"), cell("乙")];
        let rows = vec![vec![
            cell("短"),
            cell("任何能执行命令的语言都行，比如 bash、Python"),
        ]];
        let aligns = [Alignment::None, Alignment::None];
        // 宽 39：第二列收到 30 列，「…比如」正好占满，后面那个空格是满了的那一格。
        let lines: Vec<String> = render(&head, &rows, &aligns, 39)
            .iter()
            .map(plain)
            .collect();
        assert!(
            lines.iter().all(|l| l.width() == lines[0].width()),
            "每行一样宽：{lines:#?}"
        );
    }

    #[test]
    fn too_wide_tables_shrink_the_widest_column_down_to_the_floor() {
        assert_eq!(bounded(vec![30, 5], 30), vec![18, 5]);
        assert_eq!(bounded(vec![30, 30], 20), vec![14, 14], "保底两列各 14");
    }

    /// 2026-11-02 项目主人：二十列的表右边截掉了。收到保底宽度还放不下的一行一段，一格一行「表头：值」，段间一条横线。
    #[test]
    fn a_table_that_cannot_fit_is_written_one_row_per_paragraph() {
        let heads = ["臂", "次", "过", "严格", "超时", "未做", "请求", "输入M"];
        let head: Vec<Vec<Piece>> = heads.iter().map(|h| cell(h)).collect();
        let row = |name: &str| -> Vec<Vec<Piece>> {
            std::iter::once(cell(name))
                .chain((1..heads.len()).map(|i| cell(&i.to_string())))
                .collect()
        };
        let rows = vec![row("miyu 极简"), row("dsh 极简")];
        let aligns = vec![Alignment::None; heads.len()];
        let lines: Vec<String> = render(&head, &rows, &aligns, 40)
            .iter()
            .map(plain)
            .collect();
        assert!(
            !lines.iter().any(|l| l.contains('┌')),
            "不画表格：{lines:#?}"
        );
        assert!(lines.iter().all(|l| l.width() <= 40), "{lines:#?}");
        assert_eq!(lines[0], "臂：miyu 极简", "{lines:#?}");
        assert_eq!(lines[3], "严格：3");
        assert_eq!(lines[heads.len()], "─".repeat(16), "段间横线：{lines:#?}");
        assert_eq!(lines[heads.len() + 1], "臂：dsh 极简");
        assert_eq!(lines.len(), heads.len() * 2 + 1, "表头那一行不另写");
        // 放得下的照旧画表格。
        let fits = render(&head[..2], &[row("a")[..2].to_vec()], &aligns[..2], 40);
        assert!(plain(&fits[0]).starts_with('┌'));
    }
}
