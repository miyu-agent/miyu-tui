//! 回到底部（蓝图 `tui.md`「正文」第 9 条，2026-10-18 项目主人照 Claude Code 改）：正文不在底部时（人滚上去了、长正文
//! 替人停在开头），正文区最底下那一行右边浮着一段暗色的「跳转到底部（Ctrl+End）↓」，盖住底下那几格字；底下有新的字以后
//! 强调色；悬停变亮、指针变手，点了回到底部。原来是输入框右边框外面的一个 `↓`，不容易注意到。

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Span;
use ratatui::widgets::Clear;
use unicode_width::UnicodeWidthStr;

use crate::app::{Screen, Seen};
use crate::theme;

/// 浮在哪：正文区最底下那一行、贴右边，`width` 列宽；正文区放不下的是 `None`。
pub fn place(body: Rect, width: u16) -> Option<Rect> {
    let y = body.bottom().checked_sub(1)?;
    (body.height > 0 && width <= body.width).then(|| Rect::new(body.right() - width, y, width, 1))
}

/// 不在底部时画，交回它在哪（不画的是空的）。
pub fn draw(frame: &mut Frame, seen: &Seen, shown: &Screen) -> Rect {
    if shown.view.top.is_none() {
        return Rect::default();
    }
    let text = format!(" {} ", seen.config.text.to_bottom);
    let width = u16::try_from(text.width()).unwrap_or(u16::MAX);
    let Some(at) = place(shown.areas.body, width) else {
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
