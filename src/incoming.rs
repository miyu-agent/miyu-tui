//! 主循环等的东西（从 `main.rs` 分出来）：终端的事件、终端回的话、核心的消息、做好的图、叫醒；一条条交给界面。

use ratatui::crossterm::event::Event;

use crate::app::App;
use crate::core::Update;
use crate::figures;

/// 主循环等的东西：终端的事件，或者核心的消息。
pub enum Incoming {
    Terminal(Event),
    /// 终端往输入里回的一句话（OSC，`reader/replies.rs`）：点了的通知这类。
    Reply(String),
    Core(Update),
    Figure(figures::Done),
    /// 只是叫醒：跟随壁纸盯着的文件变了，这一拍再找（`theme/wallpaper.rs`）。
    Wake,
}

/// 收一条：终端的事件、核心的消息、做好的图。
pub fn dispatch(app: &mut App, message: Incoming) {
    match message {
        Incoming::Terminal(event) => app.handle(event),
        Incoming::Reply(text) => app.terminal_reply(&text),
        Incoming::Core(update) => app.core(update),
        Incoming::Figure(done) => app.media.figures.borrow_mut().done(done),
        Incoming::Wake => app.woken(),
    }
}
