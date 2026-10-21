//! 「AI 怎么认识你」那一步（「第一次打开的引导」第 23b 条）：三格（回复语言、称呼、更多描述）和「下一步」；更多描述的大编辑浮窗浮在上面（同建人格），
//! 回复语言的选择窗在格子下面（同主题那一步，`theme_step::picker`）。

use unicode_width::UnicodeWidthStr;

use super::content::Content;
use super::persona::{first_line, sheet_body};
use crate::oobe::Texts;
use crate::oobe::profile::{ABOUT, LANGUAGE, NAME, ProfileStep, Slot};
use crate::theme;

/// 三格和「下一步」。
pub fn content(step: &ProfileStep, texts: &Texts, width: u16) -> Content {
    let words = &texts.profile;
    let mut c = Content::new(width);
    c.cue = texts.keys.enter_edit.clone();
    c.heading(&words.head.title, &words.head.sub);
    let label_w = words.fields.iter().map(|f| f.width()).max().unwrap_or(0);
    // 浮窗开着时光标在浮窗里，这一屏不亮哪一格。
    let quiet = step.busy || step.writing.is_some() || step.picking.is_some();
    let on = |slot| !quiet && step.focus == slot;
    // 回复语言（核心 P-9 补，2026-10-16 项目主人定排第一）：没设的暗色「不设」；「其他…」手写时在这一格里打字。
    c.field(
        &words.fields[LANGUAGE],
        label_w,
        &step.language,
        &words.hints[LANGUAGE],
        on(Slot::Language),
    );
    // 格子之间空一行（同第 21 条）。
    c.blank();
    c.field(
        &words.fields[NAME],
        label_w,
        &step.name,
        &words.hints[NAME],
        on(Slot::Name),
    );
    c.blank();
    c.chip(
        &words.fields[ABOUT],
        label_w,
        &first_line(step.about.text()),
        &words.hints[ABOUT],
        on(Slot::About),
    );
    c.blank();
    let next = if step.blank() {
        &words.skip
    } else {
        &words.next
    };
    c.action(on(Slot::Next), &format!("{next} {}", super::next_mark()));
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
    let body = sheet_body(sheet, &words.hints[ABOUT], size);
    Some((
        words.fields[ABOUT].clone(),
        body,
        texts.keys.writing.clone(),
    ))
}
