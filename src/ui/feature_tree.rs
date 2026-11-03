//! 功能树画出来（蓝图 `tui.md`「配置页」第 38 条「功能树」，核心设计 52）：配置页的预设窗、引导的自定义、引导的「看全」
//! 共用。两列：左列照最宽的那一格补齐、空四格接右列；放不下两列的排一列（交回 `two`，按键照它走）。一格：组名加粗、后面
//! 暗色「开着的/一共」；功能 `[*] 名字`；工具缩进四格对齐在名字下面、暗色；没装的暗着、后面黄字；什么都没有的组写暗色
//! 「还没有…」。光标那一格照各处自己的样子（打头的记号、铺底色）画在格子里，不铺整行。

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::config::FeatureTreeTexts;
use crate::features::{self, Grid, Row, Tree};
use crate::theme;

/// 两列之间空几格。
const GAP: usize = 4;

/// 光标那一格怎么画：打头的记号（连同它占的那一格）和铺的底色。
pub struct Focus {
    /// 打头的记号，占两格（不够的补空格）。
    pub lead: Span<'static>,
    /// 格子铺的底色。
    pub bg: Style,
}

/// 画好的：一行行、光标在第几行（滚动照它）、排成了两列没有。
pub struct Drawn {
    /// 一行行，已经接好两列。
    pub lines: Vec<Line<'static>>,
    /// 光标那一格在第几行。
    pub focus: Option<usize>,
    /// 排成了两列。
    pub two: bool,
}

/// 画：`cursor` 是光标停的那一行（只读的「看全」是 `None`），`width` 是能用的宽。
pub fn draw(
    tree: &Tree,
    cursor: Option<Row>,
    width: u16,
    (texts, focus): (&FeatureTreeTexts, &Focus),
) -> Drawn {
    let cell = |row: Row| cell(tree, row, texts);
    let wide = |grid: &Grid, c: usize| {
        grid.cols.get(c).map_or(0, |col| {
            col.iter()
                .map(|r| spans_width(&cell(*r)))
                .max()
                .unwrap_or(0)
        })
    };
    let two = Grid::new(tree, true);
    let need = 2 + wide(&two, 0) + GAP + 2 + wide(&two, 1);
    let (grid, is_two) = if two.cols.len() > 1 && need <= usize::from(width) {
        (two, true)
    } else {
        (Grid::new(tree, false), false)
    };
    let left_w = wide(&grid, 0);
    let rows = grid.cols.iter().map(Vec::len).max().unwrap_or(0);
    let mut lines = Vec::with_capacity(rows);
    let mut at = None;
    for i in 0..rows {
        let mut spans = Vec::new();
        for (c, col) in grid.cols.iter().enumerate() {
            let row = col.get(i).copied();
            let here = row.is_some() && row == cursor;
            if here {
                at = Some(i);
            }
            let content = row.map(&cell).unwrap_or_default();
            // 左列补齐到最宽那一格、再空几格接右列；右列、一列的不补。
            let (fill, gap) = if c == 0 && grid.cols.len() > 1 {
                (left_w.saturating_sub(spans_width(&content)), GAP)
            } else {
                (0, 0)
            };
            if here {
                let lead = &focus.lead;
                let lead_pad = " ".repeat(2usize.saturating_sub(lead.content.width()));
                spans.push(Span::styled(
                    format!("{}{lead_pad}", lead.content),
                    lead.style.patch(focus.bg),
                ));
                spans.extend(content.into_iter().map(|s| s.patch_style(focus.bg)));
                spans.push(Span::styled(" ".repeat(fill), focus.bg));
            } else {
                spans.push(Span::raw("  "));
                spans.extend(content);
                spans.push(Span::raw(" ".repeat(fill)));
            }
            spans.push(Span::raw(" ".repeat(gap)));
        }
        lines.push(Line::from(spans));
    }
    // 光标停在一列最后一个停得上的行：窗照这一列的最底下滚，后面停不上的（空的组、写了没装的）也露出来。
    let focus = cursor.and_then(|row| {
        let (c, _) = grid.find(row)?;
        let col = &grid.cols[c];
        let last = grid.down(tree, row).is_none();
        Some(if last {
            col.len().saturating_sub(1)
        } else {
            at?
        })
    });
    Drawn {
        lines,
        focus: focus.or(at),
        two: is_two,
    }
}

/// 一格里写的（不带打头那两格）。
fn cell(tree: &Tree, row: Row, texts: &FeatureTreeTexts) -> Vec<Span<'static>> {
    match row {
        Row::Section(s) => {
            let (on, all) = features::tally(tree, s);
            vec![
                Span::styled(
                    tree.sections[s].name.clone(),
                    Style::new().add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("  {on}/{all}"), theme::dim()),
            ]
        }
        Row::Empty(s) => {
            let kind = &tree.sections[s].kind;
            let text = texts.empty.get(kind).or_else(|| texts.empty.get("other"));
            vec![Span::styled(
                text.cloned().unwrap_or_default(),
                theme::dim(),
            )]
        }
        Row::Blank => Vec::new(),
        Row::Feature(i) => {
            let feature = &tree.features[i];
            if !feature.installed {
                return vec![
                    Span::styled(format!("    {}", feature.name), theme::dim()),
                    Span::raw("  "),
                    Span::styled(texts.missing.clone(), theme::warn()),
                ];
            }
            let (mark, name) = if feature.on {
                (theme::accent(), Style::new())
            } else {
                (theme::dim(), theme::dim())
            };
            vec![
                Span::styled(features::mark(feature).text(), mark),
                Span::raw(" "),
                Span::styled(feature.name.clone(), name),
            ]
        }
        Row::Tool(i, j) => {
            let feature = &tree.features[i];
            let tool = &feature.tools[j];
            let on = features::tool_on(feature, tool);
            let (text, style) = if on {
                ("[*]", theme::accent())
            } else {
                ("[ ]", theme::dim())
            };
            vec![
                Span::raw("    "),
                Span::styled(text, style),
                Span::raw(" "),
                Span::styled(tool.label.clone(), theme::dim()),
            ]
        }
    }
}

/// 几段字一共多宽。
fn spans_width(spans: &[Span]) -> usize {
    spans.iter().map(|s| s.content.width()).sum()
}

#[cfg(test)]
mod tests;
