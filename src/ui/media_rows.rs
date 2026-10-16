//! 她发出去的图（核心 X-19，蓝图「她发的图」）：和回答里的图一样画在正文里，表情包照 `figures.json` 的 `sticker_room`
//! 小一些；图照 `blob.get` 读回来存成的文件画（和链接卡片、`read` 的图同一份账），没回来是「正在渲染图片」那一行。终端
//! 显示不了图的、读不成的、画不成的写一行图片色的「\[表情包\] 一句描述」（没有描述的写文件名）。

use ratatui::text::Span;

use super::figure_rows;
use super::rows::{Ctx, Row, clip};
use crate::core::Sent;
use crate::markdown::{Figure, FigureKind};
use crate::theme;
use crate::transcript::Entry;

/// 正文里这一条排成的行。
pub fn rows(entry: &Entry, ctx: &Ctx) -> Vec<Row> {
    let Some(sent) = &entry.media else {
        return Vec::new();
    };
    if !ctx.figures.borrow().shows() {
        return vec![label(sent, ctx)];
    }
    let file = ctx
        .cards
        .borrow_mut()
        .file(&sent.blob)
        .map(|p| p.display().to_string());
    let Some(source) = file else {
        if ctx.cards.borrow().unreadable(&sent.blob) {
            return vec![label(sent, ctx)];
        }
        return vec![figure_rows::pending(Vec::new(), ctx)];
    };
    let rows = figure_rows::rows(Vec::new(), &figure(sent, source), ctx);
    if rows.is_empty() {
        return vec![label(sent, ctx)];
    }
    rows
}

/// 交给画图那一套的：表情包照 `sticker_room` 的大小，别的图和回答里的一样；大小照图本身。
fn figure(sent: &Sent, source: String) -> Figure {
    Figure {
        kind: if sent.sticker {
            FigureKind::Sticker
        } else {
            FigureKind::Image
        },
        source,
        fallback: Vec::new(),
        width: None,
        height: None,
    }
}

/// 画不成图时那一行：`[表情包] 描述`，图片色，放不下截掉。
fn label(sent: &Sent, ctx: &Ctx) -> Row {
    let texts = &ctx.config.text.sent;
    let head = if sent.sticker {
        &texts.sticker
    } else {
        &texts.image
    };
    let text = match sent.alt.as_ref().or(sent.name.as_ref()) {
        Some(alt) => format!("{head} {alt}"),
        None => head.clone(),
    };
    let text = clip(&text, ctx.width);
    ctx.row(
        ctx.blank_slot(),
        vec![Span::styled(text, theme::md_image())],
    )
}

#[cfg(test)]
mod tests;
