//! 伪终端里的端到端测试：有的终端把退格键发成 `Ctrl+H`（0x08），当成退格（`reader.rs` 的 `backspace_from_ctrl_h`）。
//! 原来 crossterm 读成 `Ctrl` 加 `h`，哪都不认，删不了字（2026-11-25 查出）。

mod support;

use std::time::Duration;

use miyu_session::testkit::Script;
use support::Home;

#[test]
fn control_h_deletes_like_backspace() {
    let home = Home::new(Script::new([]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    tui.type_text("abc");
    tui.key(b"\x08");
    tui.pump(Duration::from_millis(200));
    assert_eq!(
        tui.input(),
        "ab",
        "Ctrl+H 删掉了一个字：\n{}",
        tui.lines().join("\n")
    );
}
