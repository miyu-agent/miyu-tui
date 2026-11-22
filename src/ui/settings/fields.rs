//! 编辑窗里一项怎么写（蓝图「配置页」第 14、17 到 19 条）：值（打码、继承的暗着写、选项、勾选）、右边的来源、选中时的说明。

use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

use super::fit;
use crate::settings::Texts;
use crate::settings::forms::{Field, Form, PACKAGE, Row, Val};
use crate::theme;

/// 一项的值写成几段字，和正在改时光标在值里的第几列。
pub(super) fn value(
    row: &Row,
    form: &Form,
    texts: &Texts,
    focused: bool,
    width: usize,
) -> (Vec<Span<'static>>, u16) {
    let dim = theme::dim();
    match &row.val {
        Val::Text { text, secret } => {
            if let Some(editor) = form.editing.as_ref().filter(|_| focused) {
                let typed = editor.text();
                let shown = if *secret {
                    crate::sheet::mark(|m| &m.secret).repeat(typed.chars().count())
                } else {
                    typed.to_string()
                };
                let before = &typed[..editor.cursor().min(typed.len())];
                let at = if *secret {
                    before.chars().count()
                } else {
                    before.width()
                };
                // 打着的字照普通字画，光标指出位置（2026-10-19 项目主人：打字时有下划线）。
                return (vec![Span::raw(shown)], at as u16);
            }
            if !text.is_empty() {
                let shown = if *secret {
                    crate::sheet::mark(|m| &m.secret).repeat(8)
                } else {
                    fit(text, width)
                };
                return (vec![Span::raw(shown)], 0);
            }
            let hint = match row.inherit.as_deref() {
                Some(name @ ("key_set" | "key_unset")) => {
                    texts.hint(name).unwrap_or(name).to_string()
                }
                Some(inherited) => inherited.to_string(),
                None => texts
                    .hint(row.field.name())
                    .or(texts.hint("empty"))
                    .unwrap_or_default()
                    .to_string(),
            };
            (vec![Span::styled(fit(&hint, width), dim)], 0)
        }
        Val::Choice { options, at } => {
            let current = at.map_or("", |i| options[i].as_str());
            let label = if row.field == Field::Effort && current.is_empty() {
                texts.option("effort_default")
            } else {
                texts.option(current)
            };
            // 包给的、还没动的（接口照包给的那一个停着）也暗着写，同包给的字（核心 FX-11：包给的值是默认值，人能改）。
            let given = row
                .source
                .as_deref()
                .is_some_and(|s| s.starts_with(PACKAGE))
                && form
                    .initial
                    .iter()
                    .any(|r| r.field == row.field && r.val == row.val);
            let style = if at.is_none() || given {
                dim
            } else {
                Style::new()
            };
            if focused {
                let arrow = theme::accent();
                (
                    vec![
                        Span::styled("‹ ", arrow),
                        Span::styled(label.to_string(), style),
                        Span::styled(" ›", arrow),
                    ],
                    0,
                )
            } else {
                (vec![Span::styled(label.to_string(), style)], 0)
            }
        }
        Val::Multi { options, on } => {
            let inherited = row.inherit.clone().unwrap_or_default();
            let mut spans = Vec::new();
            for (j, option) in options.iter().enumerate() {
                let ticked = match on {
                    Some(on) => on[j],
                    None => inherited.split(',').any(|h| h == option),
                };
                let mut style = if on.is_none() { dim } else { Style::new() };
                if focused && form.cursor == j {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }
                let mark = if ticked { "[x] " } else { "[ ] " };
                spans.push(Span::styled(
                    format!("{mark}{}", texts.option(option)),
                    style,
                ));
                spans.push(Span::raw("  "));
            }
            (spans, 0)
        }
        Val::Member {
            reference, lost, ..
        } => {
            let order = form.members.iter().position(|m| m == reference);
            let mark = order.map_or("[ ]".to_string(), |i| format!("[{}]", i + 1));
            let gone_text = texts.lost(*lost);
            let style = if gone_text.is_empty() {
                Style::new()
            } else {
                theme::error()
            };
            (
                vec![
                    Span::raw(format!("{mark} ")),
                    Span::styled(format!("{reference}{gone_text}"), style),
                ],
                0,
            )
        }
        Val::Fixed(text) => {
            let shown = texts.hint(text).unwrap_or(text).to_string();
            (vec![Span::styled(fit(&shown, width), dim)], 0)
        }
        Val::Section => (Vec::new(), 0),
    }
}

/// 右边写的来源：自己写了的黄字「自定义」，继承的暗着写从哪来，软件包给的写「来自软件包 X」；成员写「图」。
pub(super) fn source(row: &Row, texts: &Texts) -> (String, Style) {
    if let Val::Member { sees: true, .. } = row.val {
        return (texts.tags[0].clone(), theme::dim());
    }
    // 软件包给的（核心 PV-4）：「来自软件包 X」，同别的来源暗着写。
    if let Some(name) = row.source.as_deref().and_then(|s| s.strip_prefix(PACKAGE)) {
        let said = texts.source("package").replace("{name}", name);
        return (said, theme::dim());
    }
    // 价格那几行不写「默认」也不写「自定义」（2026-10-19 项目主人）：一列一样的字没有信息。
    let priced = matches!(
        row.field,
        Field::Input
            | Field::Output
            | Field::CacheRead
            | Field::CacheWrite
            | Field::Currency
            | Field::Multiplier
    );
    if row.own() && row.source.is_some() {
        if priced {
            return (String::new(), Style::new());
        }
        return (texts.source("own").to_string(), theme::warn());
    }
    match &row.source {
        Some(from) if priced && from == "default" => (String::new(), Style::new()),
        Some(from) => (texts.source(from).to_string(), theme::dim()),
        None => (String::new(), Style::new()),
    }
}
