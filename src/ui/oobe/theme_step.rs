//! 主题那一步的那一块（「第一次打开的引导」第 23a 条）：明暗、配色两格（光标停着的右端暗色写「Enter 选择」），跟随头像的
//! 下面暗色写照谁取色；人格没头像的再一格吉祥物，下面暗色写一句；再是「下一步 →」；在一格上按 `Enter` 开的选择窗和配置页选值的同一个（`ui/floating.rs`，2026-10-14
//! 项目主人）。

use ratatui::Frame;
use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

use super::super::floating;
use super::content::Content;
use crate::caret::Caret;
use crate::config::Config;
use crate::oobe::persona::Colors;
use crate::oobe::theme::Slot;
use crate::oobe::{Oobe, Texts};
use crate::settings::pages::choose::Choose;
use crate::theme;
use crate::theme::choice::COLORS;

/// 两三格和「下一步」。
pub fn content(oobe: &Oobe, texts: &Texts, width: u16) -> Content {
    let words = &texts.theme;
    let step = &oobe.theme;
    let mut c = Content::new(width);
    c.cue = texts.keys.enter_pick.clone();
    c.heading(&words.head.title, &words.head.sub);
    // 列几格（人格有头像的没有吉祥物那一格），名字照列着的最宽的对齐。
    let shown = if step.avatar { 2 } else { 3 };
    let label_w = words.rows[..shown]
        .iter()
        .map(|r| r.width())
        .max()
        .unwrap_or(0);
    let on = |slot| !step.busy && step.slot == slot;
    let mode = words.modes.get(step.mode).map_or("", String::as_str);
    c.chip(&words.rows[0], label_w, mode, "", on(Slot::Mode));
    // 格子之间空一行，同别的几步的表单（2026-10-14 项目主人报：明暗、配色挤在一起）。
    c.blank();
    let colors = words.colors.get(step.colors).map_or("", String::as_str);
    c.chip(&words.rows[1], label_w, colors, "", on(Slot::Colors));
    // 跟随头像的：名字下面对齐写照谁取色。
    if COLORS.get(step.colors) == Some(&"persona") {
        let persona = &oobe.persona;
        let name = persona
            .chosen
            .as_deref()
            .or(persona.existing_name.as_deref())
            .unwrap_or_default();
        let said = match persona.colors() {
            Some(Colors::Given { seed: Some(_), .. }) => &words.from_seed,
            Some(_) => &words.from_avatar,
            None => &words.no_avatar,
        };
        let lead = " ".repeat(2 + label_w + 2);
        c.note(
            &format!("{lead}{}", said.replace("{name}", name)),
            theme::dim(),
        );
    }
    // 跟随壁纸的：照样对齐写取自哪（找不到的照 Tokyo Night）。
    if COLORS.get(step.colors) == Some(&"wallpaper") {
        let said = if step.wallpaper {
            &words.from_wallpaper
        } else {
            &words.no_wallpaper
        };
        let lead = " ".repeat(2 + label_w + 2);
        c.note(&format!("{lead}{said}"), theme::dim());
    }
    // 吉祥物（2026-10-15 项目主人定）：显示、不显示，下面对齐暗色写项目主人定的那一句。
    if !step.avatar {
        c.blank();
        let shown = &words.mascot[usize::from(step.mascot_off)];
        c.chip(&words.rows[2], label_w, shown, "", on(Slot::Mascot));
        let lead = " ".repeat(2 + label_w + 2);
        c.note(&format!("{lead}{}", words.mascot_note), theme::dim());
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

/// 明暗、配色的选择窗：和配置页选值的同一个，宽照配置页的（`layout.json` 的 `settings`），放在内容那一栏（`content`）、这一步
/// 的 `rows` 行下面；后面不压暗、不盖吉祥物，挪的时候看得见整个界面当场换成什么样。
pub fn picker(
    frame: &mut Frame,
    (content, rows): (Rect, u16),
    choose: &Choose,
    config: &Config,
    caret: &mut Caret,
) {
    let look = &config.layout.settings;
    let width = look.popup_wide.min(content.width);
    let top = (content.y + rows + 1).min(content.bottom());
    let mut lines = floating::Lines::default();
    let hints = config.text.settings.more.hints[2].clone();
    floating::choose(&mut lines, choose, hints, width, |i| i);
    let height = lines.height(width, content.bottom().saturating_sub(top));
    let area = Rect::new(content.x, top, width, height);
    let mut hits = Vec::new();
    floating::centered(
        frame,
        area,
        (width, height),
        &mut lines,
        (&mut hits, |i| i),
        caret,
    );
}
