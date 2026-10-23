//! 选项带文字画时，画选中那一项的（蓝图「确认和提问的抽屉」第 3 条，2026-10-07 项目主人定）：并排时竖线落在抽屉的
//! 中线上，左半是选项、右半是预览，和选项顶上对齐；第一行暗色写「预览」，下面是图，不描边、不铺底色。图占的高度照这一页最高的那张，换选项时
//! 抽屉不跳；抽屉的字少于 `side_min` 列时挪到选项下面。

use ratatui::text::{Line, Span};
use serde::Deserialize;
use unicode_width::UnicodeWidthStr;

use super::View;
use super::items::{self, Block};
use crate::drawer::{Drawer, Item, Texts};
use crate::theme;

/// 预览面板怎么摆（`layout.json` 的 `drawer_preview`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Look {
    /// 抽屉的字够这么多列才左右并排，不够的面板放在选项下面。
    pub side_min: u16,
    /// 并排时选项和预览之间空几列，竖线画在正中。
    pub gap: u16,
    /// 中间那条竖线的字。
    pub divider: String,
    /// 「预览」和图离这一块左边几列。
    pub pad: u16,
    /// 预览最多露几行（不算「预览」那一行）：折好以后更长的能滚（`Ctrl+J` `Ctrl+K`）。
    pub rows: usize,
}

/// 选项那一块：这一页有文字画时带上选中那一项的预览。
pub fn body(d: &Drawer, texts: &Texts, look: &Look, width: u16, v: &mut View) {
    let Some(pictures) = pictures(d) else {
        items::choices(d, texts, width).append_to(v);
        return;
    };
    let shown = current(d);
    if width < look.side_min {
        items::choices(d, texts, width).append_to(v);
        v.lines.push(Line::default());
        v.items.push(None);
        let rows = tallest(&pictures, inner(look, width), look) + 1;
        let panel = panel(d, shown, texts, look, width, rows);
        v.items.extend(panel.iter().map(|_| None));
        v.lines.extend(panel);
        return;
    }
    // 选项只占它自己要的宽（最多一半），剩下的都给预览（2026-10-23 项目主人照 Claude Code：命令太宽时抽屉、预览框跟着变宽，
    // 选项往左上让位置；再放不下的折行，太长的 `Ctrl+J` `Ctrl+K` 滚）。
    let left = options_width(d, texts, width);
    let right = width.saturating_sub(left + look.gap);
    let list = items::choices(d, texts, left);
    let rows = tallest(&pictures, inner(look, right), look) + 1;
    let panel = panel(d, shown, texts, look, right, rows);
    side_by_side(list, panel, left, look).append_to(v);
}

/// 这一页每个选项的预览；一个都没有的是 `None`。
fn pictures(d: &Drawer) -> Option<Vec<&str>> {
    let list: Vec<&str> = d
        .question_at(d.tab)?
        .options
        .iter()
        .filter_map(|o| o.preview.as_deref())
        .collect();
    (!list.is_empty()).then_some(list)
}

/// 选中那一项的预览（不是选项、没有预览的是空的）。
fn current(d: &Drawer) -> &str {
    match d.current() {
        Some(Item::Choice(i)) => d
            .question_at(d.tab)
            .and_then(|q| q.options[i].preview.as_deref())
            .unwrap_or(""),
        _ => "",
    }
}

/// 预览的字能占几列（`width` 是预览那一块的宽）。
fn inner(look: &Look, width: u16) -> u16 {
    width.saturating_sub(look.pad.saturating_mul(2)).max(1)
}

/// 照 `room` 列折好的一行行：放不下的行折到下一行（Claude Code 也折，2026-10-23 项目主人）。
fn wrapped(picture: &str, room: u16) -> Vec<String> {
    let mut out = Vec::new();
    for line in picture.lines() {
        let parts = crate::input::pieces(line, room);
        if parts.is_empty() {
            out.push(String::new());
        }
        out.extend(parts.into_iter().map(|(piece, _)| piece));
    }
    out
}

/// 这一页最高的那张折好以后几行，最多 [`Look::rows`]：换选项时抽屉不跳。
fn tallest(pictures: &[&str], room: u16, look: &Look) -> usize {
    pictures
        .iter()
        .map(|p| wrapped(p, room).len())
        .max()
        .unwrap_or(0)
        .min(look.rows)
}

/// 选中那一项的预览折好以后一共几行、一次露几行（`Drawer::command_rows`、`command_window`：`Ctrl+J` `Ctrl+K` 滚的同一套）。
/// 这一页没有预览的是 `(0, 0)`。
pub fn scroll(d: &Drawer, texts: &Texts, look: &Look, width: u16) -> (usize, usize) {
    if pictures(d).is_none() {
        return (0, 0);
    }
    let right = if width < look.side_min {
        width
    } else {
        width.saturating_sub(options_width(d, texts, width) + look.gap)
    };
    (wrapped(current(d), inner(look, right)).len(), look.rows)
}

/// 选项那一列要多宽：照最宽的那一行（最多一半，至少能放下一个选项的记号和几个字）。
fn options_width(d: &Drawer, texts: &Texts, width: u16) -> u16 {
    let half = width / 2;
    let list = items::choices(d, texts, half);
    let widest = list.lines.iter().map(Line::width).max().unwrap_or(0);
    u16::try_from(widest)
        .unwrap_or(half)
        .clamp(half.min(16), half)
}

/// 这一页有预览时整个抽屉的字想要多宽：选项最宽的一行、空隙、预览最宽的一行（这一页所有选项的预览里最宽的，换选项时
/// 抽屉不跳）连两边的空。没有预览的是 `None`（照输入框的宽）。`most` 是最多能给多宽。
pub fn wanted(d: &Drawer, texts: &Texts, look: &Look, most: u16) -> Option<u16> {
    let question = d.question_at(d.tab)?;
    let widest = question
        .options
        .iter()
        .filter_map(|o| o.preview.as_deref())
        .flat_map(str::lines)
        .map(UnicodeWidthStr::width)
        .max()?;
    let panel = widest + usize::from(look.pad) * 2;
    let left = usize::from(options_width(d, texts, most));
    u16::try_from(left + usize::from(look.gap) + panel).ok()
}

/// 左边的选项和右边的预览并排成一块：左边的每一行补空格补到 `left` 列，空隙正中一条暗色竖线，再接预览。
fn side_by_side(list: Block, panel: Vec<Line<'static>>, left: u16, look: &Look) -> Block {
    let rows = list.lines.len().max(panel.len());
    let mut out = Block {
        focus: list.focus,
        cursor: list.cursor,
        ..Block::default()
    };
    let mut lines = list.lines.into_iter();
    let mut marks = list.items.into_iter();
    let mut panel = panel.into_iter();
    for _ in 0..rows {
        let mut spans = lines.next().map(|l| l.spans).unwrap_or_default();
        let used: usize = spans.iter().map(Span::width).sum();
        let half = usize::from(look.gap.saturating_sub(1) / 2);
        let pad = usize::from(left).saturating_sub(used) + half;
        spans.push(Span::raw(" ".repeat(pad)));
        spans.push(Span::styled(look.divider.clone(), theme::dim()));
        let rest = usize::from(look.gap).saturating_sub(half + look.divider.width());
        spans.push(Span::raw(" ".repeat(rest)));
        spans.extend(panel.next().map(|l| l.spans).unwrap_or_default());
        out.lines.push(Line::from(spans));
        out.items.push(marks.next().flatten());
    }
    out
}

/// 预览排成 `rows` 行：第一行暗色「预览」（下面还有的接「下面还有 N 行」），接着是折好的图从滚到的地方露起（不够的补空行，
/// 换选项时高度不变）。
fn panel(
    d: &Drawer,
    picture: &str,
    texts: &Texts,
    look: &Look,
    width: u16,
    rows: usize,
) -> Vec<Line<'static>> {
    let room = inner(look, width);
    let all = wrapped(picture, room);
    let window = rows.saturating_sub(1);
    let top = d.command_top.min(all.len().saturating_sub(window));
    let below = all.len().saturating_sub(top + window);
    let mut title = texts.preview.clone();
    if below > 0 {
        let more = texts.preview_below.replace("{n}", &below.to_string());
        title = format!("{title}{}{more}", crate::sheet::sep());
    }
    let pad = " ".repeat(usize::from(look.pad));
    let mut out = vec![Line::from(vec![
        Span::raw(pad.clone()),
        Span::styled(super::clip(&title, room), theme::dim()),
    ])];
    let mut shown = all.into_iter().skip(top);
    for _ in 0..window {
        let text = shown.next().unwrap_or_default();
        out.push(Line::from(vec![Span::raw(pad.clone()), Span::raw(text)]));
    }
    out
}
