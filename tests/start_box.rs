//! 伪终端里的端到端测试：新会话开头的人格、预设框开着时打的字、粘贴的字（蓝图 `tui.md`「新会话：人格、工作区」第 1c 条，
//! 2026-10-26 项目主人报：刚启动马上发的第一条长消息有时被吞）。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn what_is_typed_while_the_pick_box_is_open_waits_in_the_input_box() {
    let home = Home::new(Script::new([Play::Says("收到长消息。")]));
    let mut tui = home.tui_with("zh_CN.UTF-8", &[("MIYU_TUI_PRESET", "")]);
    tui.wait_for("全部功能");
    // 框开着：粘贴一段，再打几个字（打了字以后 j、k 也当字）。
    tui.send("\x1b[200~第一句很长的话\x1b[201~".as_bytes());
    tui.type_text(" jk ok");
    tui.pump(Duration::from_millis(300));
    assert!(
        tui.shows("全部功能"),
        "框还开着：\n{}",
        tui.lines().join("\n")
    );
    // 回车照旧选框里的：框关了，字还在输入框里。
    tui.key(b"\r");
    let end = Instant::now() + support::WAIT;
    while tui.shows("全部功能") {
        assert!(Instant::now() < end, "框没关：\n{}", tui.lines().join("\n"));
        tui.pump(Duration::from_millis(50));
    }
    tui.wait_for("第一句很长的话 jk ok");
    // 再回车发出去。
    tui.key(b"\r");
    tui.wait_for("收到长消息。");
}
