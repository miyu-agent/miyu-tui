//! 后台面板里点开一条在跑的命令（蓝图 `tui.md`「后台命令、子代理和侧边栏」第 3 条）：头一次要最后几行，之后照 `size`
//! 带 `after`、`wait` 接着读，长了核心才答（核心 V-6 上）。接起来的每一行只出一次、先后不乱。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn an_opened_running_command_follows_its_output_line_by_line() {
    let args = serde_json::json!({
        "command": "for i in 1 2 3 4 5 6; do echo line$i; sleep 0.4; done",
        "run_in_background": true,
        "description": "数六行"
    })
    .to_string();
    let script = Script::new([
        Play::Calls(vec![("shell".into(), args)]),
        Play::Says("开始数了。"),
        // 命令的回报当下一轮的触发消息（核心照旧开一轮）。
        Play::Says("数完了。"),
    ]);
    let home = Home::with_tools(script, "[permission]\nlevel = \"full\"\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("数一下");
    tui.wait_for("开始数了。");
    // 输入框下面的「后台」按钮：打开面板，点开那一条。
    tui.key(b"\x1b[B");
    tui.key(b"\r");
    tui.wait_for("数六行");
    tui.key(b"\r");
    tui.wait_for("line1");
    tui.wait_for("line6");
    tui.pump(Duration::from_millis(300));
    let screen = tui.lines().join("\n");
    for n in 1..=6 {
        let line = format!("line{n}");
        assert_eq!(
            screen.matches(&line).count(),
            1,
            "{line} 只出一次：\n{screen}"
        );
    }
    let at = |n: usize| screen.find(&format!("line{n}")).expect("在");
    assert!((1..6).all(|n| at(n) < at(n + 1)), "先后不乱：\n{screen}");
}
