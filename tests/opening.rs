//! 伪终端里开会话被核心拒绝（review A5，2026-10-11）：说第一句话时 `session.create` 被拒（模型、人格、预设不在了这类），
//! 那一句没发出去，照「没发出去」办：字放回输入框、弹核心的原话。原来当成「连不上核心」，连接其实好好的，之后什么都
//! 发不出去，只有断线重连才恢复。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn a_refused_new_session_puts_the_words_back_and_stays_connected() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    // 开会话照一个不存在的预设：核心一定拒。
    let mut tui = home.tui_with("zh_CN.UTF-8", &[("MIYU_TUI_PRESET", "no-such-preset")]);
    tui.wait_for("工作区");
    tui.say("你好");
    // 先等核心的原话出来（拒了），再看字回没回到输入框：刚打完字、回车还没办时输入框里本来就有这句。
    tui.wait_for("预设不存在");
    let end = Instant::now() + support::WAIT;
    while !tui.input().contains("你好") {
        assert!(
            Instant::now() < end,
            "那句话没放回输入框：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(50));
    }
    assert!(
        !tui.shows("连不上核心"),
        "连接好好的，不说连不上：\n{}",
        tui.lines().join("\n")
    );
}
