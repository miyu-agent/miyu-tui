//! 装、卸、升级的那几步画成弹窗（排期第 42 条，2026-11-04 项目主人：「回车安装软件和确认安装做成弹窗而不是混在详情那块里」）：
//! 在列表、预览块那一块的正中，盖在它们上面（背后那一块调暗），不盖最下面的状态行、按键提示。框照输入框上面那几个框（圆角、暗色框线），里面
//! 上下左右各空一格。照旧（发布前第五批审查 T5-1）：放不下的滚着看，最后一行写下面还有几行，没看完不让批（`asking_window`）。

use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::text::Line;
use ratatui::widgets::Clear;

use super::{asking_window, wrap};
use crate::settings::pages::store::{AskView, StoreTexts};
use crate::theme;

/// 弹窗最宽几列（窄的照窗口）。
const WIDTH: u16 = 72;

/// 弹窗最下面那排按钮（卸载的，PK-5）：名字、危险不危险，停着第几个；别的弹窗没有按钮。
pub(super) type Buttons<'a> = (&'a [(String, bool)], Option<usize>);

/// 画弹窗：`lines` 是那几步的字；`confirming` 的照 `ask` 滚；有按钮的钉在最下面（不跟着滚），上面空一行。画不下（窗口太矮、
/// 太窄）的交回 `None`，问的时候算没看完；画了的交回每个按钮占的地方。
pub(super) fn draw(
    frame: &mut Frame,
    stage: Rect,
    lines: Vec<Line<'static>>,
    (confirming, ask): (bool, &mut AskView),
    (buttons, selected): Buttons,
    words: &StoreTexts,
) -> Option<Vec<Rect>> {
    // 上面是面包屑、分页条（2 行起），下面留给状态行、细线、按键提示（5 行）。
    let top = stage.y + 2;
    let bottom = stage.bottom().saturating_sub(5);
    let room = bottom.saturating_sub(top);
    let width = WIDTH.min(stage.width.saturating_sub(4));
    let below: u16 = if buttons.is_empty() { 0 } else { 2 };
    if room < 5 + below || width < 12 || lines.is_empty() {
        return None;
    }
    let inner_w = width - 4;
    let max_rows = usize::from(room - 4 - below);
    let mut wrapped = wrap(&lines, inner_w);
    if confirming {
        wrapped = asking_window(ask, wrapped, max_rows, words);
    }
    let rows = wrapped.len().min(max_rows);
    let height = u16::try_from(rows).unwrap_or(0) + 4 + below;
    let outer = Rect::new(
        stage.x + (stage.width - width) / 2,
        top + (room - height) / 2,
        width,
        height,
    );
    // 背后的列表、预览块调暗：弹窗突出，两边露出来的半截字不扎眼。
    let behind = Rect::new(stage.x, top, stage.width, room);
    frame.buffer_mut().set_style(behind, theme::faint());
    frame.render_widget(Clear, outer);
    frame.buffer_mut().set_style(outer, theme::panel());
    let set = crate::sheet::read(|s| s.frame.boxes.set());
    let block = crate::ui::panel::block_with(crate::ui::panel::Chrome::default(), set);
    frame.render_widget(block, outer);
    let inner = Rect::new(outer.x + 2, outer.y + 2, inner_w, height - 4);
    for (n, line) in wrapped.iter().take(rows).enumerate() {
        let y = inner.y + u16::try_from(n).unwrap_or(0);
        frame.buffer_mut().set_line(inner.x, y, line, inner.width);
    }
    let at = Position::new(inner.x, inner.bottom().saturating_sub(1));
    Some(crate::ui::floating::button_row(
        frame, at, buttons, selected,
    ))
}
