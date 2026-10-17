//! 跑着的时候插一句再按 Ctrl+Enter（2026-10-17 项目主人报：话没发出去，再发一遍成了两条；核心 4-15 补修的）。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

#[cfg_attr(
    windows,
    ignore = "ConPTY 测具不能注入 kitty Ctrl+Enter 对应的 Win32 输入记录"
)]
#[test]
fn ctrl_enter_after_the_request_took_the_message_answers_it_once() {
    let home = Home::new(Script::new([
        Play::Stalls,
        Play::Stalls,
        Play::Says("回答插的那句。"),
    ]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("第一句");
    tui.pump(Duration::from_millis(800));
    // 她还一个字都没回：Ctrl+Enter（kitty 键盘协议）先发那句话、紧接着打断。核心把那句话带进撤掉重发的请求，打断再切断它。
    tui.type_text("插一句");
    tui.pump(Duration::from_millis(300));
    tui.key(b"\x1b[13;5u");
    tui.wait_for("回答插的那句。");
    let lines = tui.lines();
    assert_eq!(
        lines.iter().filter(|l| l.contains("插一句")).count(),
        1,
        "那句话只有一条：\n{}",
        lines.join("\n")
    );
}
