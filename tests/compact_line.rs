//! 伪终端里的端到端测试：会话的压缩触发线（核心 CT-1，`/compact-line`）。`/compact-line 70` 交核心钉住，再打 `/compact-line`
//! 说「压缩线 70%（这个会话）」；`/compact-line off` 回到跟着设置，再打说「（跟着设置）」。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 等 `text` 从屏幕上消失。
fn until_gone(tui: &mut Tui, text: &str) {
    let end = Instant::now() + support::WAIT;
    while tui.shows(text) {
        assert!(
            Instant::now() < end,
            "「{text}」还在：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(50));
    }
}

#[test]
fn compact_line_pins_the_session_line_and_off_follows_settings_again() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("/compact-line 70");
    tui.wait_for("会话尚未创建，无法设压缩线");
    tui.say("你好");
    tui.wait_for("好。");
    tui.wait_for_done();
    tui.say("/compact-line 70");
    tui.wait_for("这个会话的压缩线改成 70%");
    tui.say("/compact-line");
    tui.wait_for("压缩线 70%（这个会话）");
    tui.say("/compact-line 99");
    tui.wait_for("压缩线要在 20 到 95 之间");
    tui.say("/compact-line off");
    tui.wait_for("压缩线回到跟着设置（85%）");
    until_gone(&mut tui, "压缩线 70%（这个会话）");
    tui.say("/compact-line");
    tui.wait_for("压缩线 85%（跟着设置）");
}
