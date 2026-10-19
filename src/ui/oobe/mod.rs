//! 画引导（蓝图 `tui.md`「第一次打开的引导」第 6–12 条）：欢迎页、好了那一屏在 `screens.rs`；中间几步顶上进度、吉祥物在左、
//! 内容在右（`stage.rs` 排位置，每一步的内容在同名文件），换一步时两块一起滑。

mod content;
mod figure;
mod model;
mod more;
mod persona;
mod pick;
mod picture;
mod popup;
mod preset;
mod profile;
mod screens;
mod stage;
mod stars;
mod swap;
mod theme_step;

use std::time::{Duration, Instant};

use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::app::{MascotState, Screen};
use crate::config::Config;
use crate::oobe::{Oobe, Step};
use content::Content;
use figure::Extra;
pub use figure::idle_look;

/// 引导画的时候要的（蓝图 `app.md` 第三节）：配置、图的账只读；引导自己（弹窗滚到哪）、上一帧画面（光标）、吉祥物
/// 能改。
pub struct Parts<'a> {
    pub config: &'a Config,
    pub media: &'a crate::app::Media,
    pub oobe: &'a mut Oobe,
    pub screen: &'a mut Screen,
    pub mascot: &'a mut MascotState,
}

/// 引导开着时整屏归它。
pub fn draw(frame: &mut Frame, p: &mut Parts<'_>) {
    let now = Instant::now();
    let area = frame.area();
    let Some(step) = Some(&*p.oobe).map(|o| o.step) else {
        return;
    };
    match step {
        Step::Welcome => screens::welcome(frame, area, p, now),
        Step::Done => screens::done(frame, area, p, now),
        _ => stage_frame(frame, area, p, now),
    }
}

/// 一步的内容，`rect` 是内容那一块的大小。
fn content(step: Step, oobe: &Oobe, p: &Parts<'_>, rect: Rect, now: Instant) -> Content {
    let width = rect.width;
    let texts = &p.config.text.oobe;
    match step {
        Step::Language => pick::language(oobe, texts, width),
        Step::Icons => {
            let nerd = p.config.icon_sets.iter().find(|s| s.name == "nerd");
            pick::icons(oobe, texts, nerd, width)
        }
        Step::Model => {
            let frames = &p.config.timeline.spinner;
            let beat = now.saturating_duration_since(oobe.born).as_millis()
                / u128::from(p.config.timeline.spinner_ms.max(1));
            let spinner = frames
                .get(beat as usize % frames.len().max(1))
                .map_or("", String::as_str);
            model::content(&oobe.model, texts, (width, rect.height), spinner)
        }
        Step::Persona => persona::content(&oobe.persona, texts, width, p.config.oobe.avatar_rows),
        Step::Theme => theme_step::content(oobe, texts, width),
        Step::Profile => profile::content(&oobe.profile, texts, width),
        Step::Preset => preset::content(&oobe.preset, texts, width),
        Step::Semantic => pick::semantic(oobe, texts, &p.config.layout.bar, width),
        Step::Welcome | Step::Done => Content::new(width),
    }
}

/// 这一屏最下面写什么按键。
fn hint(oobe: &Oobe, p: &Parts<'_>) -> String {
    let keys = &p.config.text.oobe.keys;
    use crate::oobe::model::{Phase, Slot};
    use crate::oobe::persona::Slot as Field;
    match oobe.step {
        Step::Model if oobe.model.more.is_some() => keys.more.clone(),
        Step::Model => match &oobe.model.phase {
            Phase::Form(form) if form.editing() => keys.editing.clone(),
            Phase::Form(form) if form.slot() == Some(Slot::Driver) => keys.driver.clone(),
            Phase::Form(form) if form.slot() == Some(Slot::Test) => keys.test.clone(),
            Phase::Form(form) if form.slot() == Some(Slot::Anyway) => keys.anyway.clone(),
            Phase::Form(_) | Phase::Testing(_) => keys.form.clone(),
            Phase::Models(models) if models.filter.editing() => keys.searching.clone(),
            Phase::Models(_) | Phase::Saving(_) => keys.models.clone(),
            _ => keys.pick.clone(),
        },
        Step::Persona if oobe.persona.name.editing() => keys.editing.clone(),
        Step::Persona if oobe.persona.focus == Field::Next => keys.pick.clone(),
        Step::Persona if oobe.persona.focus == Field::Name => keys.persona.clone(),
        Step::Persona => keys.edit.clone(),
        Step::Theme if oobe.theme.picking.is_some() => {
            joined(&p.config.text.settings.more.hints[2])
        }
        Step::Theme if oobe.theme.slot == crate::oobe::theme::Slot::Next => keys.pick.clone(),
        Step::Theme => keys.theme.clone(),
        Step::Profile if oobe.profile.picking.is_some() => {
            joined(&p.config.text.settings.more.hints[2])
        }
        Step::Profile if oobe.profile.name.editing() || oobe.profile.language.editing() => {
            keys.editing.clone()
        }
        Step::Profile if oobe.profile.focus == crate::oobe::profile::Slot::Next => {
            keys.pick.clone()
        }
        Step::Profile => keys.edit.clone(),
        // 浮窗开着：照浮窗里那一行（`preset.rs` 的 `popup`）。
        Step::Preset if oobe.preset.viewing => keys.view.clone(),
        Step::Preset if oobe.preset.custom_open => {
            let p = &oobe.preset;
            if p.name.editing() {
                keys.editing.clone()
            } else if p.inner == 0 {
                keys.custom_name.clone()
            } else if p.inner == p.create_row() {
                keys.create.clone()
            } else {
                keys.custom.clone()
            }
        }
        Step::Preset => keys.presets.clone(),
        // 当场下语义模型的时候按键都不认：不写按键（第 26b 条）。
        Step::Semantic if oobe.semantic.installing() => String::new(),
        _ => keys.pick.clone(),
    }
}

/// 配置页的按键提示（一对对「键、做什么」）连成一行。
fn joined(pairs: &[[String; 2]]) -> String {
    pairs
        .iter()
        .map(|[k, d]| format!("{k} {d}"))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// 中间几步。
fn stage_frame(frame: &mut Frame, area: Rect, p: &mut Parts<'_>, now: Instant) {
    let Some(oobe) = Some(&*p.oobe) else {
        return;
    };
    let look = p.config.oobe.clone();
    let places = stage::places(area, &look, &p.config.mascot);
    stage::bar(frame, places.bar, oobe.step, &p.config.text.oobe.steps);
    // 没连上核心：按键提示那一行换成黄字（第 4 条）。
    if oobe.online {
        stage::hints(frame, places.hints, &hint(oobe, p));
    } else {
        stage::warn(frame, places.hints, &p.config.text.oobe.offline);
    }
    let built = content(oobe.step, oobe, p, places.content, now);
    let total = Duration::from_millis(look.slide_ms);
    let sliding = oobe.sliding.and_then(|(from, slide)| {
        slide
            .offsets(now, total, places.content.width)
            .map(|o| (from, o))
    });
    let mut focus = None;
    match sliding {
        Some((from, (old, new))) => {
            let before = content(from, oobe, p, places.content, now);
            place(frame, places.content, &before, old);
            place(frame, places.content, &built, new);
        }
        None => {
            let shift = place(frame, places.content, &built, 0);
            // 头像的小图（第 21 条）：滑动时不画。
            if let Some(picture) = &built.picture {
                picture::draw(frame, &p.media.figures, places.content, shift, picture);
            }
            let row = |r: u16| places.content.y + r.saturating_sub(shift);
            if let Some((r, col)) = built.caret {
                let at = Position::new(places.content.x + col, row(r));
                if places.content.contains(at) {
                    p.screen.caret.put(at, true);
                    focus = Some(at);
                }
            }
            focus = focus.or_else(|| {
                built
                    .focus
                    .map(|r| Position::new(places.content.x + 2, row(r)))
            });
        }
    }
    let Some(rect) = places.mascot else {
        popup_frame(frame, area, p, true);
        return;
    };
    let Some(oobe) = Some(&*p.oobe) else {
        return;
    };
    let act = oobe.react.act(now, &look.react);
    let badge = oobe.badge.clone();
    let rect = Rect::new(
        rect.x,
        rect.y.saturating_sub(act.lift),
        rect.width,
        rect.height,
    );
    let extra = Extra {
        mouth: act.mouth,
        ear: act.ear,
        blink: act.blink,
        yaw: act.yaw,
        pitch: act.pitch,
        roll: act.roll,
        ..Extra::default()
    };
    let reach = f64::from(
        places
            .content
            .right()
            .saturating_sub(rect.x + rect.width / 2),
    );
    // 浮窗开着时吉祥物看浮窗里的光标。
    let popup_caret = popup_caret(frame, area, p);
    // 吉祥物、头像，或者什么都不画；要画的变了当场换（`swap.rs`，第 11a 条）。
    let gaze = (popup_caret.or(focus), reach);
    swap::draw(frame, (area, rect, act.lift), p, gaze, extra, now);
    // 人格建好了：名字一个字一个字写到它脚下（第 10 条）。
    if let Some((name, at)) = badge {
        let per = Duration::from_millis(look.react.name_ms.max(1));
        let shown = (now.saturating_duration_since(at).as_millis() / per.as_millis() + 1) as usize;
        let text: String = name.chars().take(shown).collect();
        let y = rect.bottom() + act.lift;
        if y < area.bottom() {
            let line = Line::styled(text, crate::theme::dim());
            frame.render_widget(
                Paragraph::new(line).centered(),
                Rect::new(rect.x, y, rect.width, 1),
            );
        }
    }
    popup_frame(frame, area, p, true);
}

/// 开着浮窗（大编辑浮窗、示范对话、更多供应商）：先算出浮窗里的光标给吉祥物看（浮窗盖在它上面，画在后面）。
fn popup_caret(frame: &mut Frame, area: Rect, p: &mut Parts<'_>) -> Option<Position> {
    let (shown, caret) = popup_frame(frame, area, p, false);
    shown.then_some(caret).flatten()
}

/// 画浮窗（第 16a、21a 条）；`paint` 是假的只算光标。交回开没开、输入光标在哪。
fn popup_frame(
    frame: &mut Frame,
    area: Rect,
    p: &mut Parts<'_>,
    paint: bool,
) -> (bool, Option<Position>) {
    // 主题那一步的选择窗：和配置页选值的同一个（第 23a 条）。
    if let Some(choose) = p
        .oobe
        .theme
        .picking
        .as_ref()
        .filter(|_| p.oobe.step == Step::Theme)
    {
        if paint {
            // 放在内容那一栏、两格下面：不盖吉祥物，挪的时候看得见整个界面换成什么样。
            let places = stage::places(area, &p.config.oobe, &p.config.mascot);
            let built = theme_step::content(p.oobe, &p.config.text.oobe, places.content.width);
            let below = (places.content, built.len());
            theme_step::picker(frame, below, choose, p.config, &mut p.screen.caret);
        }
        return (true, None);
    }
    // 「关于你」那一步回复语言的选择窗：同主题那一步，放在格子下面（第 23b 条）。
    if let Some(choose) = p
        .oobe
        .profile
        .picking
        .as_ref()
        .filter(|_| p.oobe.step == Step::Profile)
    {
        if paint {
            let places = stage::places(area, &p.config.oobe, &p.config.mascot);
            let built =
                profile::content(&p.oobe.profile, &p.config.text.oobe, places.content.width);
            let below = (places.content, built.len());
            theme_step::picker(frame, below, choose, p.config, &mut p.screen.caret);
        }
        return (true, None);
    }
    let look = p.config.oobe.popup.clone();
    let texts = p.config.text.oobe.clone();
    let dialogs = p.config.text.settings.more.dialogs.clone();
    let more = &p.config.text.settings.more.hints;
    let hints = (joined(&more[8]), joined(&more[9]));
    let rect = popup::rect(area, &look);
    let inner = popup::body(rect);
    let Some(oobe) = Some(&mut *p.oobe) else {
        return (false, None);
    };
    let size = (inner.width, inner.height);
    let built = match oobe.step {
        Step::Persona => persona::popup(
            &mut oobe.persona,
            &texts,
            &dialogs,
            (&hints.0, &hints.1),
            size,
        ),
        // 「更多供应商…」（第 16a 条）。
        Step::Model => oobe
            .model
            .more
            .as_ref()
            .map(|m| more::popup(m, &texts, size)),
        // 看预设、自定义（第 24、26 条）。
        Step::Preset => preset::popup(&mut oobe.preset, &texts, size),
        // 更多描述的大编辑浮窗（第 23b 条）。
        Step::Profile => profile::popup(&mut oobe.profile, &texts, size),
        _ => None,
    };
    let Some((title, body, hint)) = built else {
        return (false, None);
    };
    if !paint {
        let caret = body
            .caret
            .map(|(r, c)| Position::new(inner.x + c, inner.y + r));
        return (true, caret);
    }
    let caret = popup::draw(frame, area, rect, (&title, &body, &hint));
    if let Some(at) = caret {
        p.screen.caret.put(at, true);
    }
    (true, caret)
}

/// 把一块画进 `rect`，往右挪 `shift` 列（往左是负的），出了 `rect` 的裁掉；比 `rect` 高的照光标那一行往上滚。交回滚了几行。
fn place(frame: &mut Frame, rect: Rect, built: &Content, shift: i32) -> u16 {
    let len = u16::try_from(built.lines.len()).unwrap_or(u16::MAX);
    let focus = built.caret.map(|c| c.0).or(built.focus).unwrap_or(0);
    let scroll = if len > rect.height {
        (focus + 3)
            .saturating_sub(rect.height)
            .min(len - rect.height)
    } else {
        0
    };
    let (x, width, skip) = if shift >= 0 {
        let s = u16::try_from(shift).unwrap_or(u16::MAX).min(rect.width);
        (rect.x + s, rect.width - s, 0)
    } else {
        let s = u16::try_from(-shift).unwrap_or(u16::MAX).min(rect.width);
        (rect.x, rect.width - s, s)
    };
    if width == 0 {
        return scroll;
    }
    let target = Rect::new(x, rect.y, width, rect.height);
    frame.render_widget(
        Paragraph::new(built.lines.clone()).scroll((scroll, skip)),
        target,
    );
    scroll
}

#[cfg(test)]
mod tests;
