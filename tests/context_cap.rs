//! 伪终端里的端到端测试：会话的上下文上限（核心 CTX-1、CTX-2，设计 68，CTX-3）。`/context 128k` 交核心设上限，底栏
//! 环后面写「会话上限 128k」；`/context off` 去掉，那一截跟着没了。

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
fn context_with_a_number_caps_the_session_and_the_footer_says_so() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("你好");
    tui.wait_for("好。");
    tui.say("/context 128k");
    tui.wait_for("会话上限 128k");
    tui.say("/context off");
    until_gone(&mut tui, "会话上限");
}
