//! 语言、图标、沙盒几步的那一块（「第一次打开的引导」第 13、14、26a 条）。

use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::content::Content;
use crate::config::Bar;
use crate::config::Icons;
use crate::oobe::semantic::download::{Offer, Phase};
use crate::oobe::{Oobe, SemanticTexts, Texts};
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

/// 开不开内置的语义模型（`oobe/semantic.rs`，2026-10-18 项目主人定）：一个标题、一段说明、「是」「否」两行。没装的「是」后面
/// 写要下多大；在下的两行换成进度条，装不上的写原话、两行换成「重试」「跳过」（2026-10-19，第 26b 条）。
pub fn semantic(oobe: &Oobe, texts: &Texts, bar: &Bar, width: u16) -> Content {
    let step = &oobe.semantic;
    let words = &texts.semantic;
    let mut c = Content::new(width);
    c.heading(&words.head.title, &words.head.sub);
    let rows: [(String, String); 2] = match &step.phase {
        Phase::Installing { done, total } => {
            c.lines.push(progress(*done, *total, words, bar, width));
            return c;
        }
        Phase::Failed { said, why } => {
            c.note(said, theme::error());
            if let Some(why) = why {
                c.note(why, theme::dim());
            }
            c.blank();
            let d = &words.download;
            [
                (d.retry.clone(), String::new()),
                (d.skip.clone(), String::new()),
            ]
        }
        Phase::Choose => {
            let size = match step.offer {
                Some(Offer::Download(Some(bytes))) if step.installed == Some(false) => Some(
                    words
                        .download
                        .size
                        .replace("{size}", &crate::drawer::size(bytes)),
                ),
                _ => None,
            };
            let on = match size {
                Some(size) => format!("{} · {size}", words.on[1]),
                None => words.on[1].clone(),
            };
            [
                (words.on[0].clone(), on),
                (words.off[0].clone(), words.off[1].clone()),
            ]
        }
    };
    for (i, (label, note)) in rows.into_iter().enumerate() {
        c.row(
            i == step.pick.selected,
            vec![
                Span::raw(label),
                Span::raw("  "),
                Span::styled(note, theme::dim()),
            ],
            None,
        );
    }
    if let Some(error) = &step.error {
        c.blank();
        c.note(error, theme::error());
    }
    c
}

/// 下载的进度条（同界面块的进度条的两种格），后面写百分比和下了多少；还没推来进度的写「准备下载…」。
fn progress(done: u64, total: u64, words: &SemanticTexts, bar: &Bar, width: u16) -> Line<'static> {
    let d = &words.download;
    if total == 0 {
        return Line::styled(d.preparing.clone(), theme::dim());
    }
    let value = (done as f64 / total as f64).clamp(0.0, 1.0);
    let tail = d
        .progress
        .replace("{percent}", &format!("{:.0}", value * 100.0))
        .replace("{done}", &crate::drawer::size(done))
        .replace("{total}", &crate::drawer::size(total));
    let room = usize::from(width).saturating_sub(tail.width() + 2);
    let cells = bar.width.min(room).max(1);
    let full = ((value * cells as f64).round() as usize).min(cells);
    let mut spans: Vec<Span<'static>> = (0..cells)
        .map(|i| {
            if i < full {
                Span::styled(bar.full.clone(), theme::accent())
            } else {
                Span::styled(bar.empty.clone(), theme::dim())
            }
        })
        .collect();
    spans.push(Span::styled(format!("  {tail}"), theme::dim()));
    Line::from(spans)
}
