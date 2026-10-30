//! 选预设那一步的那一块（「第一次打开的引导」第 24–26 条）：一个预设一行，最后一行「自定义」。空格看一个预设的功能、
//! 工具，回车（空格）开自定义，都是浮窗（[`popup`]）。

use ratatui::style::Modifier;
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

use super::content::Content;
use crate::config::FeatureTreeTexts;
use crate::oobe::Texts;
use crate::oobe::preset::PresetStep;
use crate::theme;
use crate::ui::feature_tree::{self, Focus};

/// 照读回来的拼：一块两行，选着的两行都铺底，右边 `✓`。
pub fn content(step: &PresetStep, texts: &Texts, width: u16) -> Content {
    let words = &texts.preset;
    let mut c = Content::new(width);
    c.heading(&words.head.title, &words.head.sub);
    if step.busy && step.cards.is_empty() {
        c.note(&words.loading, theme::dim());
        return c;
    }
    // 一个预设一行（第 24 条，2026-10-10 项目主人：空格看得到详情，下面那一行功能不用写了）。
    for (i, card) in step.cards.iter().enumerate() {
        let selected = i == step.cursor;
        let mark = selected.then(|| Span::styled(crate::sheet::mark(|m| &m.ok), theme::accent()));
        // 名字后面暗色接描述，同「自定义」那一行（2026-10-27 项目主人报：原来只有自定义有描述）。
        let mut spans = vec![Span::styled(card.name.clone(), bold())];
        if let Some(summary) = &card.summary {
            spans.push(Span::styled(format!("  {summary}"), theme::dim()));
        }
        c.row(selected, spans, mark);
    }
    let custom = step.custom();
    c.row(
        custom,
        vec![
            Span::styled(words.custom.clone(), bold()),
            Span::styled(format!("  {}", words.custom_note), theme::dim()),
        ],
        custom.then(|| Span::styled(crate::sheet::mark(|m| &m.ok), theme::accent())),
    );
    if let Some(error) = step.error.as_ref().filter(|_| !step.custom_open) {
        c.blank();
        c.note(error, theme::error());
    }
    c
}

/// 开着的浮窗：标题、里面的一块、按键提示；没开的是 `None`。看的浮窗只列功能树（只读），照 `view_top` 滚；自定义的浮窗是
/// 名字、功能树的开关、「创建 →」（第 24、26 条；功能树照「配置页」第 38 条，核心设计 52）。
pub fn popup(
    step: &mut PresetStep,
    texts: &Texts,
    (width, height): (u16, u16),
    tree: &FeatureTreeTexts,
) -> Option<(String, Content, String)> {
    let words = &texts.preset;
    // 光标那一格：同列表选着的那一行，打头的记号、铺底色（`content.rs` 的 `row`）。
    let focus = Focus {
        lead: Span::styled(super::content::pointer(), theme::accent()),
        bg: theme::row_focus(),
    };
    if step.viewing {
        let card = step.cards.get(step.cursor)?;
        let list = card.features.clone().unwrap_or_default();
        let mut c = Content::new(width);
        // 最上面先写描述、空一行，再列功能（2026-10-27 项目主人报：空格打开的详情没有描述）。
        let mut height = height;
        if let Some(summary) = &card.summary {
            c.note(summary, theme::dim());
            c.blank();
            height = height.saturating_sub(u16::try_from(c.lines.len()).unwrap_or(u16::MAX));
        }
        let rows = feature_tree::draw(&list, None, width.saturating_sub(2), (tree, &focus)).lines;
        // 放不下的上下各留一行暗色「还有 N 行」，滚到底就停（照搜模型的列表）。
        let height = usize::from(height.max(3));
        let room = if rows.len() > height {
            height - 2
        } else {
            height
        };
        step.view_top = step.view_top.min(rows.len().saturating_sub(room));
        let (above, below) = (&texts.model.more_above, &texts.model.more_below);
        let note = |text: &str, n: usize| {
            ratatui::text::Line::styled(
                format!("  {}", text.replace("{n}", &n.to_string())),
                theme::faint(),
            )
        };
        if rows.len() > height {
            c.lines.push(if step.view_top > 0 {
                note(above, step.view_top)
            } else {
                ratatui::text::Line::default()
            });
        }
        let shown = rows.len().min(step.view_top + room);
        c.lines.extend(rows[step.view_top..shown].iter().cloned());
        if shown < rows.len() {
            c.lines.push(note(below, rows.len() - shown));
        }
        return Some((card.name.clone(), c, texts.keys.view.clone()));
    }
    if !step.custom_open {
        return None;
    }
    let mut c = Content::new(width);
    c.cue = texts.keys.enter_edit.clone();
    c.on_panel = true;
    let label_w = words.name.width();
    c.field(
        &words.name,
        label_w,
        &step.name,
        &words.name_hint,
        step.inner == 0 && !step.busy,
    );
    // 名称下面空一行，不和功能挤在一起（第 26 条，2026-10-10 项目主人）。
    c.blank();
    // 功能树（「配置页」第 38 条，核心设计 52）：两列，光标那一格自己画，浮窗照它滚。
    let rows = step.rows();
    let cursor = step.inner.checked_sub(1).and_then(|i| rows.get(i)).copied();
    let drawn = feature_tree::draw(
        &step.features,
        cursor,
        width.saturating_sub(2),
        (tree, &focus),
    );
    step.two.set(drawn.two);
    if let Some(at) = drawn.focus {
        c.focus = Some(u16::try_from(c.lines.len() + at).unwrap_or(u16::MAX));
    }
    c.lines.extend(drawn.lines);
    c.blank();
    c.action(
        step.inner == step.create_row(),
        &format!("{} {}", words.create, super::next_mark()),
    );
    if let Some(error) = &step.error {
        c.blank();
        c.note(error, theme::error());
    }
    let keys = &texts.keys;
    let hint = if step.name.editing() {
        &keys.editing
    } else if step.inner == 0 {
        &keys.custom_name
    } else if step.inner == step.create_row() {
        &keys.create
    } else {
        &keys.custom
    };
    Some((words.custom.clone(), c, hint.clone()))
}

fn bold() -> ratatui::style::Style {
    ratatui::style::Style::new().add_modifier(Modifier::BOLD)
}
