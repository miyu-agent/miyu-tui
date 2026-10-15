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

/// 1×1 的 PNG。
const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

/// 核心 C-9（2026-10-15）：派子代理时交代带着附件，子会话里那句交代照人发附件那样画成块。
#[test]
fn a_task_handed_over_with_an_image_shows_it_as_a_block_in_the_agents_session() {
    let png = std::env::temp_dir().join(format!("miyu-tui-c9-{}.png", std::process::id()));
    std::fs::write(&png, PNG).expect("写得了图");
    let args = serde_json::json!({
        "description": "看图",
        "prompt": "看看这张图",
        "attachments": [png.display().to_string()],
    })
    .to_string();
    let script = Script::new([
        Play::Calls(vec![("subagent".into(), args)]),
        Play::Holds,
        Play::Holds,
    ]);
    let home = Home::with_tools(script, "");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("派个子代理看图");
    tui.wait_for("○ 看图");
    tui.key(DOWN);
    tui.key(DOWN);
    tui.key(b"\r");
    tui.wait_for("● 看图");
    // 交代那一句：字后面接着图的块。
    tui.wait_for("看看这张图 [图片 1]");
    if std::fs::remove_file(&png).is_err() {}
}
