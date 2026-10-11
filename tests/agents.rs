//! 伪终端里的端到端测试：子代理状态行接真的核心（蓝图 `tui.md`「后台命令、子代理和侧边栏」第 6 条、「切进子会话」）。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;

const DOWN: &[u8] = b"\x1b[B";

/// 切进子代理的会话以后，在它那一行按 `x` 停的是它（review B4，2026-10-11）：任务表是主会话的，停也要对着主会话发；
/// 原来对着正在看的子会话发，核心找不到这个任务编号，子代理一直跑着。
#[test]
fn stopping_an_agent_while_viewing_it_stops_that_agent() {
    let args = serde_json::json!({"description": "查资料", "prompt": "去查一下"}).to_string();
    // 主会话派完接着停住；子代理一开口就停住：两边都在跑，谁先要下一段都一样。
    let script = Script::new([
        Play::Calls(vec![("subagent".into(), args)]),
        Play::Holds,
        Play::Holds,
    ]);
    let home = Home::with_tools(script, "");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("派个子代理");
    tui.wait_for("● 主会话");
    tui.wait_for("○ 查资料");
    // 焦点下到子代理那一行，切进它的会话，再按 x。
    tui.key(DOWN);
    tui.key(DOWN);
    tui.key(b"\r");
    tui.wait_for("● 查资料");
    tui.key(b"x");
    tui.wait_for("已停止");
}
