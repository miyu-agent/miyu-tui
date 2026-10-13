//! 「AI 怎么认识你」那一步（「第一次打开的引导」第 23a 条）：两格和「下一步」；更多描述的大编辑浮窗浮在上面（同建人格）。

use unicode_width::UnicodeWidthStr;

use super::content::Content;
use super::persona::{first_line, sheet_body};
use crate::oobe::Texts;
use crate::oobe::profile::{ProfileStep, Slot};
use crate::theme;

/// 两格和「下一步」。
pub fn content(step: &ProfileStep, texts: &Texts, width: u16) -> Content {
    let words = &texts.profile;
    let mut c = Content::new(width);
    c.cue = texts.keys.enter_edit.clone();
    c.heading(&words.head.title, &words.head.sub);
    let label_w = words.fields.iter().map(|f| f.width()).max().unwrap_or(0);
    // 浮窗开着时光标在浮窗里，这一屏不亮哪一格。
    let quiet = step.busy || step.writing.is_some();
    let on = |slot| !quiet && step.focus == slot;
    c.field(
        &words.fields[0],
        label_w,
        &step.name,
        &words.hints[0],
        on(Slot::Name),
    );
    // 格子之间空一行（同第 21 条）。
    c.blank();
    c.chip(
        &words.fields[1],
        label_w,
        &first_line(step.about.text()),
        &words.hints[1],
        on(Slot::About),
    );
    c.blank();
    c.action(on(Slot::Next), &format!("{} →", words.next));
    if step.busy {
        c.blank();
        c.note(&words.saving, theme::dim());
    }
    if let Some(error) = &step.error {
        c.blank();
        c.note(error, theme::error());
    }
    c
}

/// 开着的大编辑浮窗：标题、里面的一块、按键提示；没开的是 `None`。
pub fn popup(
    step: &mut ProfileStep,
    texts: &Texts,
    size: (u16, u16),
) -> Option<(String, Content, String)> {
    let words = &texts.profile;
    let sheet = step.writing.as_mut()?;
    let body = sheet_body(sheet, &words.hints[1], size);
    Some((words.fields[1].clone(), body, texts.keys.writing.clone()))
}
