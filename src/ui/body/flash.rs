//! `/jump` 跳到的那一条亮一下（蓝图「跳到说过的话」第 3 条）：那一条露出来的每一行，竖线换成列表里选中那一条的品红
//! （强调色和工作区那一级的竖线是同一个蓝，看不出来），到点画回原来的。

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::body_view::BodyView;
use crate::theme;
use crate::transcript::Entry;
use crate::ui::row_cache::Rows;

/// 照 `view.flash` 给跳到的那一条的竖线换色；过了时候的去掉。`first` 是视口第一行是第几行，`bar` 是竖线连同它后面的
/// 空格（`layout.json` 的 `user_bar`），认格子照它去掉空白的那一个字。
pub(super) fn paint(
    buf: &mut Buffer,
    area: Rect,
    (rows, first): (&Rows, usize),
    entries: &[Entry],
    view: &mut BodyView,
    bar: &str,
) {
    let Some((id, until)) = view.flash else {
        return;
    };
    if std::time::Instant::now() >= until {
        view.flash = None;
        return;
    }
    let Some(index) = entries.iter().position(|e| e.id == id) else {
        return;
    };
    let (Some(start), Some(end)) = (rows.start_of(index), rows.end_of(index)) else {
        return;
    };
    let bar = bar.trim();
    let height = usize::from(area.height);
    for row in start.max(first)..end.min(first + height) {
        let y = area.y + u16::try_from(row - first).unwrap_or(0);
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            if cell.symbol() == bar {
                cell.set_style(theme::picked());
            }
        }
    }
}
