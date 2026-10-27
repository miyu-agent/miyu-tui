//! 画悬浮窗（蓝图「配置页」第 14、15、20 条）：浅一档的底、不画边框；标题、一行一项（名字、值、来源），选中的那项下面
//! 一行说明；按钮、按键提示。编辑窗、选模型的窗、问一句的窗三种。

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::super::floating;
use super::fields::{source, value};
use super::{Look, fit};
use crate::caret::Caret;
use crate::settings::forms::{Field, Focus, Form, Target, Val, window_text};
use crate::settings::nav::Use;
use crate::settings::popup::{Choice, Item, Pick, Popup};
use crate::settings::{Hit, Settings, Texts};
use crate::theme;

/// 名字那一栏多宽。
const LABEL: u16 = 14;

/// 画开着的悬浮窗。
pub fn draw(
    frame: &mut Frame,
    area: Rect,
    page: &mut Settings,
    texts: &Texts,
    look: &Look,
    caret: &mut Caret,
) {
    let Some(mut popup) = page.popup.take() else {
        return;
    };
    let wide = !matches!(
        &popup,
        Popup::Form(Form {
            target: Target::Provider(_),
            ..
        }) | Popup::Confirm(_)
    );
    let width = if wide {
        look.popup_wide
    } else {
        look.popup_width
    }
    .min(area.width.saturating_sub(2));
    let max_height = (area.height * look.popup_height_percent / 100)
        .max(6)
        .min(area.height);
    let mut lines = Lines::default();
    match &mut popup {
        Popup::Form(form) => form_lines(&mut lines, form, page, texts, width),
        Popup::Pick(pick) => pick_lines(&mut lines, pick, page, texts, width),
        Popup::Confirm(confirm) => {
            lines.title(confirm.title.clone(), String::new());
            for (text, dim) in &confirm.lines {
                lines.plain(Line::styled(
                    text.clone(),
                    if *dim { theme::dim() } else { Style::new() },
                ));
            }
            lines.buttons = confirm
                .buttons
                .iter()
                .map(|(b, danger)| (b.clone(), *danger))
                .collect();
            lines.button = Some(confirm.sel);
            lines.hints = texts.popup_hints[3].clone();
        }
        Popup::Choose(choose) => {
            let hints = texts.more.hints[2].clone();
            floating::choose(&mut lines, choose, hints, width, Hit::PickRow);
        }
        // 预设的名称、描述就在详情窗那一行里改，不另开小窗（2026-10-23 项目主人：「回车直接编辑就好了」）。
        Popup::Line(edit) => match &edit.target {
            crate::settings::pages::target::Target::Preset(view) => {
                super::more_popups::preset(&mut lines, view, page, texts, (width, Some(edit)));
            }
            _ => super::more_popups::line(&mut lines, edit, texts),
        },
        Popup::Sheet(edit) => {
            let room = (width.saturating_sub(6), max_height.saturating_sub(6));
            super::more_popups::sheet(&mut lines, edit, texts, room);
        }
        Popup::Persona(view) => super::more_popups::persona(&mut lines, view, page, texts),
        Popup::Preset(view) => {
            super::more_popups::preset(&mut lines, view, page, texts, (width, None));
        }
        Popup::Dialogs(view) => super::dialog_popups::dialogs(&mut lines, view, texts, width),
        Popup::List(edit) => super::dialog_popups::list(&mut lines, edit, texts, width),
        Popup::Ticks(edit) => super::dialog_popups::ticks(&mut lines, edit, texts, width),
        Popup::Pair(edit) => super::dialog_popups::pair(&mut lines, edit, texts),
        Popup::Catalog(pick) => super::catalog::lines(&mut lines, pick, texts, width),
    }
    // 窗里从第几行露起照上一帧接着（选中的碰到上下边才挪，`floating.rs`）。
    lines.top = page.popup_top;
    floating::centered(
        frame,
        area,
        (width, max_height),
        &mut lines,
        (&mut page.hits, Hit::Button),
        caret,
    );
    page.popup_top = lines.top;
    page.popup = Some(popup);
}

/// 窗里的一行行（`ui/floating.rs`）：点到的是配置页的 [`Hit`]。
pub(super) type Lines = floating::Lines<Hit>;
/// 一行。
pub(super) type Body = floating::Body<Hit>;

fn form_lines(lines: &mut Lines, form: &Form, page: &Settings, texts: &Texts, width: u16) {
    let shown = |id: &str| {
        page.view
            .provider(id)
            .map_or(id.to_string(), |p| p.shown().to_string())
    };
    let (title, sub) = match &form.target {
        Target::Provider(None) => (texts.titles[0].clone(), String::new()),
        Target::Provider(Some(id)) => {
            (texts.titles[1].replace("{name}", &shown(id)), String::new())
        }
        Target::Model(p, None) => (texts.titles[2].replace("{name}", &shown(p)), String::new()),
        Target::Model(p, Some(m)) => (m.clone(), shown(p)),
        Target::Pool(None) => (texts.titles[3].clone(), String::new()),
        Target::Pool(Some(n)) => (texts.titles[4].replace("{name}", n), String::new()),
    };
    lines.title(title, sub);
    let env = form.choice(Field::Auth) == Some("env");
    let value_width = width.saturating_sub(LABEL + 14) as usize;
    for (i, row) in form.rows.iter().enumerate() {
        let focused = form.focus == Focus::Row(i);
        if matches!(row.val, Val::Section) {
            // 一节和上面的格子之间空一行（2026-10-19 项目主人：「价格」和「温度」挤在一起）。
            if i > 0 {
                lines.plain(Line::raw(""));
            }
            lines.plain(Line::styled(
                format!(" {}", texts.field(row.field.name())),
                theme::md_heading(),
            ));
            continue;
        }
        let editing = focused && form.editing.is_some();
        let (value, caret) = value(row, form, texts, focused, value_width);
        let mut spans = Vec::new();
        if !matches!(row.val, Val::Member { .. }) {
            let name = if row.field == Field::Key && env {
                "env_key"
            } else {
                row.field.name()
            };
            let label = texts.field(name);
            let pad = (LABEL as usize).saturating_sub(label.width() + 2);
            // 选中的那一项名字加粗、用正文色，和没选中的暗字分得开（同一天项目主人：选中的不明显）。
            let style = if focused {
                Style::new().add_modifier(Modifier::BOLD)
            } else {
                theme::hover()
            };
            spans.push(Span::styled(format!("  {label}{}", " ".repeat(pad)), style));
        } else {
            spans.push(Span::raw("  "));
        }
        let start: u16 = spans.iter().map(|s| s.content.width() as u16).sum();
        spans.extend(value);
        let source = source(row, texts);
        let used: usize = spans.iter().map(|s| s.content.width()).sum();
        let right = width.saturating_sub(4) as usize;
        if !source.0.is_empty() && used + source.0.width() + 2 <= right {
            spans.push(Span::raw(" ".repeat(right - used - source.0.width())));
            spans.push(Span::styled(source.0, source.1));
        }
        if editing {
            lines.caret = Some((lines.body.len(), start + caret));
        }
        lines.body.push(Body {
            line: Line::from(spans),
            focused,
            hit: Some(Hit::FormRow(i)),
        });
    }
    lines.error = form.error.clone();
    lines.buttons = texts.buttons.iter().map(|b| (b.clone(), false)).collect();
    lines.button = match form.focus {
        Focus::Button(b) => Some(b),
        Focus::Row(_) => None,
    };
    let which = if form.editing.is_some() { 1 } else { 0 };
    lines.hints = texts.popup_hints[which].clone();
}

fn pick_lines(lines: &mut Lines, pick: &Pick, page: &Settings, texts: &Texts, width: u16) {
    let at = usize::from(pick.usage == Use::Vision);
    let sub = if pick.usage == Use::Vision {
        texts.pick[1].clone()
    } else {
        String::new()
    };
    lines.title(texts.uses[at].clone(), sub);
    let search = if pick.typing || !pick.search.is_empty() {
        Line::raw(format!(" / {}", pick.search))
    } else {
        Line::styled(format!(" {}", texts.pick[4]), theme::dim())
    };
    if pick.typing {
        lines.caret = Some((0, 3 + pick.search.width() as u16));
    }
    lines.plain(search);
    let current = match pick.usage {
        Use::Chat => page.view.chat.clone(),
        Use::Vision => page.view.vision.clone(),
        Use::Embedding => None,
    };
    let right = width.saturating_sub(4) as usize;
    for (i, item) in pick.items(&page.view).into_iter().enumerate() {
        match item {
            Item::Head(name) => {
                let name = name.unwrap_or_else(|| texts.pick[0].clone());
                lines.plain(Line::styled(format!(" {name}"), theme::md_heading()));
            }
            Item::Choice {
                reference,
                label,
                tag,
                off,
            } => {
                let mark = if current.as_deref() == Some(reference.as_str()) {
                    format!("{} ", crate::sheet::mark(|m| &m.on))
                } else {
                    "  ".to_string()
                };
                let tag = match (off, tag) {
                    (Some("off_vision"), _) => texts.pick[2].clone(),
                    (Some(_), _) => texts.pick[3].clone(),
                    (None, Choice::Pool(how, n)) => texts
                        .pool_tag
                        .replace("{how}", texts.option(how.as_deref().unwrap_or_default()))
                        .replace("{n}", &n.to_string()),
                    (None, Choice::Model(window, sees)) => {
                        let mut parts: Vec<String> = window.map(window_text).into_iter().collect();
                        if sees {
                            parts.push(texts.tags[0].clone());
                        }
                        parts.join(" ")
                    }
                };
                let style = if off.is_some() {
                    theme::dim()
                } else {
                    Style::new()
                };
                let text = fit(&label, right.saturating_sub(tag.width() + 6));
                let used = 2 + mark.width() + text.width();
                let pad = right.saturating_sub(used + tag.width());
                let line = Line::from(vec![
                    Span::raw("  "),
                    Span::styled(mark, theme::accent()),
                    Span::styled(text, style),
                    Span::raw(" ".repeat(pad)),
                    Span::styled(tag, theme::dim()),
                ]);
                lines.body.push(Body {
                    line,
                    focused: i == pick.sel,
                    hit: Some(Hit::PickRow(i)),
                });
            }
        }
    }
    lines.hints = texts.popup_hints[2].clone();
}
