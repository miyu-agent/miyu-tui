//! 新会话开头选人格、预设（蓝图 `tui.md`「新会话：人格、工作区」第 1、3 条，2026-10-21 项目主人：「不是以列表的形式出，而是
//! 直接替换掉输入框」）：画在输入框的位置，框线照输入框，上边框写「请选择人格」「请选择预设」，下边框写按键，框里上下各空
//! 一行；列表还没读回来写「正在加载…」。这时没有输入框，打不了字（`keys.rs`）。

use ratatui::text::Line;

use super::panel::{self, Chrome};
use crate::app::Panel;
use crate::app::Seen;
use crate::app::new_session::Kind;
use crate::theme;

/// 这一样（`kind`）在输入框的位置画什么：框（标题、按键）和框里的几行（连上下的空行），最多 `max` 行。
pub(super) fn content(
    seen: &Seen,
    kind: Kind,
    width: u16,
    max: usize,
) -> (Chrome, Vec<Line<'static>>) {
    let texts = &seen.config.text.persona_box;
    let pad = usize::from(panel::PAD);
    let (chrome, rows) = match seen.boxes.panel {
        Some(Panel::Pick {
            kind: open,
            selected,
        }) if open == kind => {
            let (chrome, rows, _) = super::panels::pick_rows(
                seen,
                kind,
                selected,
                width,
                max.saturating_sub(pad).max(1),
            );
            (chrome, rows)
        }
        _ => {
            let title = match kind {
                Kind::Persona => &texts.title,
                Kind::Preset => &texts.preset_title,
            };
            let loading = Line::styled(format!("  {}", texts.loading), theme::dim());
            (Chrome::new(title, Vec::new()), vec![loading])
        }
    };
    let mut lines = vec![Line::default()];
    lines.extend(rows);
    lines.push(Line::default());
    (chrome, lines)
}
