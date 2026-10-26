//! 画加供应商先弹的目录窗（蓝图「配置页」第 11a 条）：标题同编辑窗的「添加供应商」，最上面一行筛的字（同选模型的窗），
//! 下面一家一行（接不上的暗着），接着「更多供应商…」「自定义…」；还没回来的写「正在读取…」。

use ratatui::style::Style;
use ratatui::text::Line;
use unicode_width::UnicodeWidthStr;

use super::fit;
use super::popup::{Body, Lines};
use crate::settings::pages::catalog::{CatalogPick, Row};
use crate::settings::{Hit, Texts};
use crate::theme;

/// 窗里的一行行。
pub(super) fn lines(lines: &mut Lines, pick: &CatalogPick, texts: &Texts, width: u16) {
    lines.title(texts.titles[0].clone(), String::new());
    let search = if pick.typing || !pick.search.is_empty() {
        Line::raw(format!(" / {}", pick.search))
    } else {
        Line::styled(format!(" {}", texts.pick[4]), theme::dim())
    };
    if pick.typing {
        lines.caret = Some((0, 3 + pick.search.width() as u16));
    }
    lines.plain(search);
    let rows = pick.rows();
    let empty = !rows.iter().any(|r| matches!(r, Row::Provider(_)));
    match &pick.providers {
        None => lines.plain(Line::styled(format!(" {}", texts.catalog[2]), theme::dim())),
        Some(_) if empty && !pick.search.is_empty() => {
            lines.plain(Line::styled(format!(" {}", texts.catalog[3]), theme::dim()));
        }
        Some(_) => {}
    }
    let room = usize::from(width.saturating_sub(6));
    for (i, row) in rows.iter().enumerate() {
        let (label, style) = match row {
            Row::Provider(p) if p.supported => (p.name.as_str(), Style::new()),
            Row::Provider(p) => (p.name.as_str(), theme::faint()),
            Row::More => (texts.catalog[0].as_str(), Style::new()),
            Row::Custom => (texts.catalog[1].as_str(), Style::new()),
        };
        lines.body.push(Body {
            line: Line::styled(format!("  {}", fit(label, room)), style),
            focused: i == pick.sel,
            hit: Some(Hit::PickRow(i)),
        });
    }
    lines.hints = texts.popup_hints[2].clone();
}
