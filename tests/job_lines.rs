//! 伪终端里的端到端测试：监视的后台命令（排期第 37 条，核心 MN-1；蓝图 `tui.md`「后台命令、子代理和侧边栏」第 5 条子条）：
//! 命令每出一批行，正文写一行「◆ 监视 · 标题 · N 行」（标题同了结那一行，后台命令写命令本身），命令照旧在跑。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn a_watched_command_writes_a_line_for_each_batch_it_prints() {
    let args = serde_json::json!({
        "command": "echo hello-watch; sleep 10",
        "watch": true,
        "description": "盯日志"
    })
    .to_string();
    let script = Script::new([
        Play::Calls(vec![("shell".into(), args)]),
        Play::Says("盯着了。"),
        // 出了一批行：核心叫醒她，开一轮。
        Play::Says("看到了。"),
        Play::Says("结束了。"),
    ]);
    let home = Home::with_tools(script, "[permission]\nlevel = \"full\"\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("盯一下");
    tui.wait_for("盯着了。");
    tui.wait_for("◆ 监视 · echo hello-watch; sleep 10 · 1 行");
    assert!(
        !tui.shows("后台命令完成"),
        "出了行不是了结：\n{}",
        tui.lines().join("\n")
    );
}
