//! 窗口矮到文字区 0 行高（审计 2026-11-28 第 8 条）：1 行高的窗口里框是 `y = 0, h = 1`，文字区排成 `y = 1, h = 0`，
//! 点在框那一行原来 `clamp(1, 0)` panic。

use std::time::Duration;

use ratatui::crossterm::event::{MouseButton, MouseEventKind};
use ratatui::layout::Rect;

use crate::input::InputBox;

#[test]
fn clicking_a_zero_height_text_area_does_not_panic() {
    let mut i = InputBox::new(8, Duration::from_millis(400));
    i.editor.insert("还没发的字");
    i.place(Rect::new(2, 1, 10, 0));
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Drag(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        i.mouse(super::mouse(kind, 3, 0), true);
    }
    assert_eq!(i.editor.text(), "还没发的字", "字不动");
}
