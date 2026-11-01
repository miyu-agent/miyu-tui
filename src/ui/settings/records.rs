//! 记录列表的两个窗（蓝图「配置页」第 47b 条）：列表窗一条一行写它的标题；表单窗一个字段一行（名字照最长的对齐、值加粗，
//! 滑块的字段画短条，同照清单画的一页）。

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::fit;
use super::popup::{Body, Lines};
use crate::settings::Texts;
use crate::settings::pages::records::{RecordForm, RecordsEdit, label};
use crate::settings::pages::{shown, slider};
use crate::theme;

/// 列表窗：标题是这一项的名字；一条都没有的暗色写「还没有」。
pub(super) fn list(lines: &mut Lines, edit: &RecordsEdit, texts: &Texts, width: u16) {
    let words = &texts.more.records;
    lines.title(edit.item.name.clone(), String::new());
    lines.hints = words.hints.clone();
    if edit.records.is_empty() {
        lines.plain(Line::styled(format!("  {}", words.empty), theme::dim()));
        return;
    }
    let room = usize::from(width).saturating_sub(6);
    for (i, record) in edit.records.iter().enumerate() {
        let title = label(&edit.item, record, i + 1, words);
        lines.body.push(Body {
            line: Line::raw(format!("  {}", fit(&title, room))),
            focused: i == edit.sel,
            hit: None,
        });
    }
}

/// 表单窗：标题「<项名> › <这一条的标题>」，一个字段一行。
pub(super) fn form(lines: &mut Lines, form: &RecordForm, texts: &Texts, width: u16) {
    let words = &texts.more.records;
    let item = &form.list.item;
    let n = form.at.map_or(form.list.records.len() + 1, |i| i + 1);
    let record = serde_json::Value::Object(form.record.clone());
    let title = format!("{} › {}", item.name, label(item, &record, n, words));
    lines.title(title, String::new());
    lines.hints = words.form_hints.clone();
    let name_w = item
        .fields
        .iter()
        .map(|f| f.name.width())
        .max()
        .unwrap_or(0)
        + 2;
    let room = usize::from(width).saturating_sub(name_w + 8);
    for (i, field) in item.fields.iter().enumerate() {
        let value = form.record.get(&field.key).unwrap_or(&field.default);
        let pad = " ".repeat(name_w.saturating_sub(field.name.width()));
        let mut spans = vec![Span::raw(format!("  {}{pad}", field.name))];
        let text = shown(field, value, None, &texts.more);
        let bold = Style::new().add_modifier(Modifier::BOLD);
        match slider::fraction(field, value) {
            Some(at) => {
                spans.extend(super::pages::track(at));
                spans.push(Span::styled(format!("  {}", fit(&text, room)), bold));
            }
            None if value.is_null() => spans.push(Span::styled(fit(&text, room), theme::dim())),
            None => spans.push(Span::styled(fit(&text, room), bold)),
        }
        lines.body.push(Body {
            line: Line::from(spans),
            focused: i == form.sel,
            hit: None,
        });
    }
}
