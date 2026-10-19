//! 回到底部（蓝图 `tui.md`「正文」第 9 条，2026-10-18 项目主人照 Claude Code 改）：正文不在底部时（人滚上去了、长正文
//! 替人停在开头），输入框上面紧挨着的那一行空行正中浮着一段暗色的「跳转到底部（Ctrl+End）↓」（2026-10-19 项目主人改：原来
//! 在正文区最底下那一行右边，待办、运行状态行一出来就被顶上去）；底下有新的字以后强调色；悬停变亮、指针变手，点了回到底部。
//! 输入框上面开着列表、面板时那一行被盖住，不画。

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Span;
use ratatui::widgets::Clear;
use unicode_width::UnicodeWidthStr;

use crate::app::{Screen, Seen};
use crate::theme;

/// 浮在哪：输入框（连边框，`frame`）上面紧挨着的那一行、照框居中，`width` 列宽；上面开着列表、面板（`menu` 有高度）、
/// 框比这段字还窄、框顶到了最上面的是 `None`。
pub fn place(frame: Rect, menu: Rect, width: u16) -> Option<Rect> {
    if menu.height > 0 || width > frame.width {
        return None;
    }
    let y = frame.y.checked_sub(1)?;
    Some(Rect::new(frame.x + (frame.width - width) / 2, y, width, 1))
}

/// 不在底部时画，交回它在哪（不画的是空的）。
pub fn draw(frame: &mut Frame, seen: &Seen, shown: &Screen) -> Rect {
    if shown.view.top.is_none() {
        return Rect::default();
    }
    let text = format!(" {} ", seen.config.text.to_bottom);
    let width = u16::try_from(text.width()).unwrap_or(u16::MAX);
    let areas = &shown.areas;
    let Some(at) = place(areas.frame, areas.menu, width).filter(|_| areas.body.height > 0) else {
        return Rect::default();
    };
    let hovered = shown.pointer.is_some_and(|p| at.contains(p));
    let style = if hovered {
        theme::hover()
    } else if shown.view.rows.len() > shown.view.seen_end {
        theme::accent()
    } else {
        theme::dim()
    };
    frame.render_widget(Clear, at);
    frame.render_widget(Span::styled(text, style), at);
    at
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::place;

    #[test]
    fn it_sits_centered_on_the_blank_line_right_above_the_input_box() {
        // 2026-10-19 项目主人：放在输入框上面那一行空行正中，不被待办、运行状态行顶上去。
        let frame = Rect::new(10, 30, 80, 4);
        assert_eq!(
            place(frame, Rect::default(), 20),
            Some(Rect::new(40, 29, 20, 1))
        );
        let menu = Rect::new(10, 20, 80, 10);
        assert_eq!(place(frame, menu, 20), None, "列表、面板盖住那一行时不画");
        assert_eq!(place(frame, Rect::default(), 81), None, "框比字窄");
        assert_eq!(
            place(Rect::new(0, 0, 80, 4), Rect::default(), 20),
            None,
            "顶到最上面"
        );
    }
}
