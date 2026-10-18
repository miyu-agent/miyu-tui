//! 点开一步时照 `tool.result` 的 `detail`（给人看的结构化结果，核心 D-6）画的那几样（蓝图「时间线」第 8 条，2026-10-17
//! 项目主人：记忆、知识库点开原来是给模型看的英文）：翻记忆一条一行「日期 · 内容」，作废的划线，对话片段先一行
//! 「日期 · 会话 短编号」、下面接原文；查知识库一段一块「库 / 路径 · 标题」（没有标题的写第几页），下面接片段；读文档照
//! Markdown 画那一页。网页照同一套画。

use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use serde_json::Value;

use super::super::rows::Ctx;
use super::step::Piece;
use crate::config::Body;
use crate::{markdown, theme};

/// `detail` 什么都没有（没找到：核心给空列表）。
pub fn empty(body: &Body, detail: &Value) -> bool {
    let none = |key: &str| detail[key].as_array().is_some_and(Vec::is_empty);
    match body {
        Body::Memories => none("memories") && none("turns"),
        Body::Hits => none("hits"),
        _ => false,
    }
}

/// 照 `body` 把 `detail` 排成一行行；不是这几样的、`detail` 形状不对的交回 `None`（照结果的原文画）。`each` 把一段字
/// 照宽度折成行。
pub fn pieces(
    body: &Body,
    detail: &Value,
    style: Style,
    width: u16,
    ctx: &Ctx,
    each: &dyn Fn(&str, Style) -> Vec<Piece>,
) -> Option<Vec<Piece>> {
    let field = |v: &Value, key: &str| v[key].as_str().unwrap_or_default().trim().to_string();
    let gap = || Piece {
        lead: None,
        content: Vec::new(),
        joined: false,
        links: Vec::new(),
    };
    let mut out: Vec<Piece> = Vec::new();
    match body {
        Body::Memories => {
            let memories = detail["memories"].as_array()?;
            let turns = detail["turns"].as_array()?;
            for memory in memories {
                let line = format!("{} · {}", field(memory, "date"), field(memory, "text"));
                let style = if memory["forgotten"] == true {
                    style.add_modifier(Modifier::CROSSED_OUT)
                } else {
                    style
                };
                out.extend(each(&line, style));
            }
            for turn in turns {
                if !out.is_empty() {
                    out.push(gap());
                }
                let id = crate::session_list::short(&field(turn, "session"));
                let session = ctx.config.text.on_session.replace("{id}", &id);
                out.extend(each(
                    &format!("{} · {session}", field(turn, "date")),
                    theme::dim(),
                ));
                out.extend(each(&field(turn, "text"), style));
            }
        }
        Body::Hits => {
            for hit in detail["hits"].as_array()? {
                if !out.is_empty() {
                    out.push(gap());
                }
                let place = match (hit["heading"].as_str(), hit["page"].as_u64()) {
                    (Some(heading), _) if !heading.trim().is_empty() => heading.trim().to_string(),
                    (_, Some(page)) => ctx.config.text.on_page.replace("{page}", &page.to_string()),
                    _ => String::new(),
                };
                let mut head = format!("{} / {}", field(hit, "kb"), field(hit, "path"));
                if !place.is_empty() {
                    head = format!("{head} · {place}");
                }
                out.extend(each(&head, theme::dim()));
                // 片段是文档里的原文：和读文档一样照 Markdown 画，不露 `**`。
                out.extend(markdown_pieces(&field(hit, "snippet"), width, ctx));
            }
        }
        Body::Page => out.extend(markdown_pieces(detail["text"].as_str()?, width, ctx)),
        Body::None | Body::Arg(_) | Body::Diff(_) | Body::Message => return None,
    }
    Some(out)
}

/// 照 Markdown 排成一行行（图不画）。
pub(super) fn markdown_pieces(text: &str, width: u16, ctx: &Ctx) -> Vec<Piece> {
    let kit = markdown::Kit {
        languages: &ctx.config.languages,
        math: &ctx.config.math,
        labels: &ctx.config.text.markdown,
        tab: ctx.config.layout.output.tab,
    };
    markdown::render(text.trim(), width, &kit, &[])
        .into_iter()
        .filter(|line| line.figure.is_none())
        .map(|line| {
            // 引子（列表记号、引用竖线）放进内容里，链接的列照它往后挪。
            let shift =
                u16::try_from(line.lead.iter().map(Span::width).sum::<usize>()).unwrap_or(0);
            Piece {
                lead: None,
                content: line
                    .lead
                    .into_iter()
                    .chain(line.folded.spans)
                    .collect::<Vec<Span>>(),
                joined: line.folded.joined,
                links: line
                    .folded
                    .links
                    .into_iter()
                    .map(|(from, to, url)| (from + shift, to + shift, url))
                    .collect(),
            }
        })
        .collect()
}
