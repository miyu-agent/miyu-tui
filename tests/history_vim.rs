//! 按键模式是 Vim 时的输入历史列表（发版前 check 31，2026-11-09 项目主人：「/ 进入搜索模式，平时 jk 上下」；蓝图 `tui.md`
//! 「输入历史列表」）：照会话列表那一套，没按 `/` 时打的字不进搜索。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 等输入框里的字变成 `want`。
fn wait_input(tui: &mut Tui, want: &str) {
    let end = Instant::now() + support::WAIT;
    while tui.input() != want {
        assert!(
            Instant::now() < end,
            "输入框不是「{want}」：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(50));
    }
}

#[test]
fn j_and_k_move_and_a_slash_searches() {
    let script = Script::new([Play::Says("answer-one"), Play::Says("answer-two")]);
    let home = Home::with_settings(script, "[tui]\nkeys = \"vim\"\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("alpha-line");
    tui.wait_for("answer-one");
    tui.say("beta-line");
    tui.wait_for("answer-two");
    // 打开时选中最新的；k 往更早的走，j 回来，字不进搜索。
    tui.key(b"\x12");
    tui.wait_for("j/k 选择");
    tui.key(b"k");
    tui.pump(Duration::from_millis(200));
    tui.key(b"j");
    tui.pump(Duration::from_millis(200));
    tui.key(b"k");
    tui.pump(Duration::from_millis(200));
    tui.key(b"\r");
    wait_input(&mut tui, "alpha-line");
    // 清掉再开：/ 进搜索，打的字筛，Enter 收起搜索，再 Enter 填进输入框。
    tui.key(b"\x03");
    tui.pump(Duration::from_millis(200));
    tui.key(b"\x12");
    tui.wait_for("j/k 选择");
    tui.key(b"/");
    tui.wait_for("输入以搜索");
    tui.type_text("alp");
    tui.pump(Duration::from_millis(200));
    tui.key(b"\r");
    tui.wait_for("j/k 选择");
    tui.key(b"\r");
    wait_input(&mut tui, "alpha-line");
}
