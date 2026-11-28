//! 全屏开的那几种终端模式（从 `main.rs` 分出来）：鼠标、括号粘贴、焦点上报、kitty 键盘协议；退出、让出终端、崩了、收到
//! 要结束的信号（`quit_signals.rs`）时照样关上。

use std::io::{self, Write, stdout};

use ratatui::crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::{cursor, execute, terminal};

use crate::{pixel_mouse, pointer};

/// 打开鼠标、括号粘贴；终端认得 kitty 键盘协议就打开，Shift+Enter 才分得出来。
/// 返回有没有打开键盘协议，退出时照着关。光标的样子不碰，跟终端自己的设置（`tui.md`「输入框」第 4 条）。
pub fn enter() -> io::Result<bool> {
    let keyboard = terminal::supports_keyboard_enhancement().unwrap_or(false);
    on(keyboard)?;
    Ok(keyboard)
}

/// 打开鼠标、括号粘贴，`keyboard` 为真时再打开 kitty 键盘协议。
pub fn on(keyboard: bool) -> io::Result<()> {
    // 焦点上报：终端报在不在前台，系统通知照它（蓝图「系统通知」第 2 条）。
    execute!(
        stdout(),
        EnableMouseCapture,
        EnableBracketedPaste,
        EnableFocusChange
    )?;
    if keyboard {
        execute!(
            stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?;
    }
    Ok(())
}

/// 关掉 [`on`] 开的，连同指针形状、像素坐标。
pub fn leave(keyboard: bool) -> io::Result<()> {
    leave_into(&mut stdout(), keyboard)
}

/// 同 [`leave`]，写进 `out`。
fn leave_into(out: &mut impl Write, keyboard: bool) -> io::Result<()> {
    pointer::reset(out)?;
    pixel_mouse::PixelMouse::default().reset(out)?;
    if keyboard {
        execute!(out, PopKeyboardEnhancementFlags)?;
    }
    execute!(
        out,
        DisableMouseCapture,
        DisableBracketedPaste,
        DisableFocusChange
    )
}

/// 收到要结束的信号、主循环到点没退完（`quit_signals.rs`）：当场整个还原，写不进的不管。先收掉可能画到一半的同步输出，
/// 关 [`on`] 开的，亮光标、离开备用屏、退出原始模式。
pub fn restore_now(out: &mut dyn Write, keyboard: bool) {
    restore_into(out, keyboard);
}

/// 同 [`restore_now`]（crossterm 的命令要写进一个有大小的写入端）。
fn restore_into(mut out: impl Write, keyboard: bool) {
    if out.write_all(b"\x1b[?2026l").is_err() {
        // 收尾时写不进、还原不了的不管：能还原多少还原多少。
    }
    if leave_into(&mut out, keyboard).is_err() {
        // 收尾时写不进、还原不了的不管：能还原多少还原多少。
    }
    if execute!(out, cursor::Show, terminal::LeaveAlternateScreen).is_err() {
        // 收尾时写不进、还原不了的不管：能还原多少还原多少。
    }
    if terminal::disable_raw_mode().is_err() {
        // 收尾时写不进、还原不了的不管：能还原多少还原多少。
    }
}

/// shell 集成画一轮的时候（`shellhook/say/session.rs`）收到要结束的信号：当场还原它开的（kitty 键盘协议、括号粘贴、焦点
/// 上报、藏起的光标、原始模式），写不进的不管。不在备用屏里，不开鼠标。
pub fn restore_line(out: &mut dyn Write, keyboard: bool) {
    line_into(out, keyboard);
}

/// 同 [`restore_line`]。
fn line_into(mut out: impl Write, keyboard: bool) {
    if keyboard && execute!(out, PopKeyboardEnhancementFlags).is_err() {
        // 收尾时写不进、还原不了的不管：能还原多少还原多少。
    }
    if execute!(out, DisableBracketedPaste, DisableFocusChange, cursor::Show).is_err() {
        // 收尾时写不进、还原不了的不管：能还原多少还原多少。
    }
    if terminal::disable_raw_mode().is_err() {
        // 收尾时写不进、还原不了的不管：能还原多少还原多少。
    }
}
