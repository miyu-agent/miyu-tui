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

/// 派一个子代理、主会话这一轮停住，再打断主会话：主会话闲着、子代理接着跑。交回界面，停在主会话。
fn idle_main_with_a_running_agent(home: &Home) -> support::Tui {
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("派个子代理");
    tui.wait_for("○ 查资料");
    // 先照原来的两下 Esc 打断主会话这一轮：子代理照常跑（`13-终端界面.md` 第十节）。
    tui.key(b"\x1b");
    tui.key(b"\x1b");
    tui.wait_for("已中断");
    assert!(tui.shows("○ 查资料"), "打断主会话不停子代理");
    tui
}

/// 派子代理、两边都停住的剧本。
fn agent_script() -> Script {
    let args = serde_json::json!({"description": "查资料", "prompt": "去查一下"}).to_string();
    Script::new([
        Play::Calls(vec![("subagent".into(), args)]),
        Play::Holds,
        Play::Holds,
    ])
}

/// 主会话闲着、子代理还在跑：两下 Esc 停掉子代理（2026-10-25 项目主人：「esc 打断主会话以后再 esc 我期望打断子代理」）。
#[test]
fn esc_twice_in_an_idle_main_session_stops_its_agents() {
    let home = Home::with_tools(agent_script(), "");
    let mut tui = idle_main_with_a_running_agent(&home);
    tui.key(b"\x1b");
    tui.wait_for("再按一次 Esc 停止子代理");
    tui.key(b"\x1b");
    tui.wait_for("已停止");
}

/// 主会话闲着、子代理还在跑：输入框空着按 Ctrl+C 一样停掉子代理。
#[test]
fn ctrl_c_in_an_idle_main_session_stops_its_agents() {
    let home = Home::with_tools(agent_script(), "");
    let mut tui = idle_main_with_a_running_agent(&home);
    tui.key(b"\x03");
    tui.wait_for("已停止");
}

/// 切进在跑的子代理：Ctrl+C 停掉它（不只打断它这一轮），人留在它的画面里（2026-10-25 项目主人定）。
#[test]
fn ctrl_c_while_viewing_an_agent_stops_it_and_stays() {
    let home = Home::with_tools(agent_script(), "");
    let mut tui = idle_main_with_a_running_agent(&home);
    tui.key(DOWN);
    tui.key(DOWN);
    tui.key(b"\r");
    tui.wait_for("● 查资料");
    // 焦点回输入框（↑ 两下），再按 Ctrl+C。
    tui.key(b"\x1b[A");
    tui.key(b"\x1b[A");
    tui.key(b"\x03");
    tui.wait_for("已停止");
    assert!(
        tui.shows("子代理 · 查资料"),
        "还在子代理的画面里：\n{}",
        tui.lines().join("\n")
    );
}

/// 切进在跑的子代理：两下 Esc 停掉它，人留在它的画面里；停了以后再按 Esc 才回主会话。
#[test]
fn esc_twice_while_viewing_a_running_agent_stops_it_and_stays() {
    let home = Home::with_tools(agent_script(), "");
    let mut tui = idle_main_with_a_running_agent(&home);
    tui.key(DOWN);
    tui.key(DOWN);
    tui.key(b"\r");
    tui.wait_for("● 查资料");
    // 焦点还在状态行上：第一下 Esc 先回输入框（由近及远），再两下停。
    tui.key(b"\x1b");
    tui.key(b"\x1b");
    tui.wait_for("再按一次 Esc 停止子代理");
    assert!(tui.shows("子代理 · 查资料"), "第一下不回主会话");
    tui.key(b"\x1b");
    tui.wait_for("已停止");
    assert!(tui.shows("子代理 · 查资料"), "停了还留在它的画面里");
    tui.pump(std::time::Duration::from_millis(1600));
    tui.key(b"\x1b");
    tui.pump(std::time::Duration::from_millis(500));
    assert!(!tui.shows("子代理 · 查资料"), "停了以后 Esc 回主会话");
}
