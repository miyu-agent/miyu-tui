//! 主题那一步的那一块（「第一次打开的引导」第 23a 条）：明暗、配色两行左右切换，跟随头像的下面暗色写照谁取色，再是
//! 「下一步 →」。

use unicode_width::UnicodeWidthStr;

use super::content::Content;
use crate::oobe::persona::Colors;
use crate::oobe::theme::Slot;
use crate::oobe::{Oobe, Texts};
use crate::theme;
use crate::theme::choice::COLORS;

/// 两行和「下一步」。
pub fn content(oobe: &Oobe, texts: &Texts, width: u16) -> Content {
    let words = &texts.theme;
    let step = &oobe.theme;
    let mut c = Content::new(width);
    c.heading(&words.head.title, &words.head.sub);
    let label_w = words.rows.iter().map(|r| r.width()).max().unwrap_or(0);
    let on = |slot| !step.busy && step.slot == slot;
    let mode = words.modes.get(step.mode).map_or("", String::as_str);
    c.switch(&words.rows[0], label_w, mode, on(Slot::Mode));
    let colors = words.colors.get(step.colors).map_or("", String::as_str);
    c.switch(&words.rows[1], label_w, colors, on(Slot::Colors));
    // 跟随头像的：名字下面对齐写照谁取色。
    if COLORS.get(step.colors) == Some(&"persona") {
        let persona = &oobe.persona;
        let name = persona
            .chosen
            .as_deref()
            .or(persona.existing_name.as_deref())
            .unwrap_or_default();
        let said = match persona.colors() {
            Some(Colors::Seed(_)) => &words.from_seed,
            Some(_) => &words.from_avatar,
            None => &words.no_avatar,
        };
        let lead = " ".repeat(2 + label_w + 2);
        c.note(
            &format!("{lead}{}", said.replace("{name}", name)),
            theme::dim(),
        );
    }
    c.blank();
    c.action(on(Slot::Next), &format!("{} →", words.next));
    if step.saving() {
        c.blank();
        c.note(&words.saving, theme::dim());
    }
    if let Some(error) = &step.error {
        c.blank();
        c.note(error, theme::error());
    }
    c
}
