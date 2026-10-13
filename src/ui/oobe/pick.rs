//! 语言、图标、沙盒几步的那一块（「第一次打开的引导」第 13、14、26a 条）。

use ratatui::style::Modifier;
use ratatui::text::{Line, Span};

use super::content::Content;
use crate::config::Icons;
use crate::oobe::{Oobe, Texts};
use crate::theme;

/// 语言：第一行跟随系统，下面一种一行。
pub fn language(oobe: &Oobe, texts: &Texts, width: u16) -> Content {
    let words = &texts.language;
    let mut c = Content::new(width);
    c.heading(&words.head.title, &words.head.sub);
    for (i, row) in oobe.language.rows.iter().enumerate() {
        c.row(
            i == oobe.language.selected,
            vec![Span::raw(row.clone())],
            None,
        );
    }
    c
}

/// 图标：中间一行间隔着的几个图标（照 `nerd` 那一套，窄得放不下的照放得下的个数），下面两行选项。
pub fn icons(oobe: &Oobe, texts: &Texts, nerd: Option<&Icons>, width: u16) -> Content {
    let words = &texts.icons;
    let mut c = Content::new(width);
    c.heading(&words.head.title, &words.head.sub);
    if let Some(set) = nerd {
        let gap = "    ";
        let mut row = String::new();
        for glyph in oobe.look.sample_icons.iter().map(|t| set.tool(t)) {
            let next = if row.is_empty() {
                glyph.to_string()
            } else {
                format!("{row}{gap}{glyph}")
            };
            if unicode_width::UnicodeWidthStr::width(next.as_str()) > usize::from(width) {
                break;
            }
            row = next;
        }
        let pad = usize::from(width)
            .saturating_sub(unicode_width::UnicodeWidthStr::width(row.as_str()))
            / 2;
        c.lines.push(Line::from(vec![
            Span::raw(" ".repeat(pad)),
            Span::styled(row, theme::accent().add_modifier(Modifier::BOLD)),
        ]));
        c.blank();
    }
    for (i, label) in [&words.yes, &words.no].into_iter().enumerate() {
        c.row(
            i == oobe.icons.selected,
            vec![Span::raw(label.clone())],
            None,
        );
    }
    c
}

/// 沙盒：两行，「开」「关」，后面暗色一句是什么意思；写不成的红字。
pub fn sandbox(oobe: &Oobe, texts: &Texts, width: u16) -> Content {
    let words = &texts.sandbox;
    let mut c = Content::new(width);
    c.heading(&words.head.title, &words.head.sub);
    for (i, [label, note]) in [&words.on, &words.off].into_iter().enumerate() {
        c.row(
            i == oobe.sandbox.pick.selected,
            vec![
                Span::raw(label.clone()),
                Span::raw("  "),
                Span::styled(note.clone(), theme::dim()),
            ],
            None,
        );
    }
    if let Some(error) = &oobe.sandbox.error {
        c.blank();
        c.note(error, theme::error());
    }
    c
}
