//! 伪终端里的端到端测试：会话的上下文上限（核心 CTX-1、CTX-2，设计 68，CTX-3）。`/context 128k` 交核心设上限，`/status`
//! 的上下文那一行写「会话上限 128k」（底栏不写，2026-11-11 项目主人）；`/context off` 去掉，那一截跟着没了。

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
fn context_with_a_number_caps_the_session_and_status_says_so() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("你好");
    tui.wait_for("好。");
    tui.wait_for_done();
    tui.say("/context 128k");
    // 底栏不写上限了（2026-11-11 项目主人），看 `/status` 的上下文那一行。
    tui.say("/status");
    tui.wait_for("会话上限 128k");
    tui.key(b"\x1b");
    until_gone(&mut tui, "会话 ID");
    assert!(
        !tui.shows("会话上限 128k"),
        "底栏不写：\n{}",
        tui.lines().join("\n")
    );
    tui.say("/context off");
    tui.say("/status");
    tui.wait_for("会话 ID");
    until_gone(&mut tui, "会话上限 128k");
}
