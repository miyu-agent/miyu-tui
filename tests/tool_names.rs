//! 伪终端里的端到端测试：工具的显示名（蓝图 `tui.md`「给人看的字」，核心 9-8 六补、七补）。还在跑的那一步照核心给的工具表
//! （`human.get`，七补起有脚本、MCP、扩展工具）；跑完的照结果里记下的显示名，照界面语言挑；工具卸了、重开会话照日志补发，
//! 照样是记下的那个名字。

mod support;

use miyu_kernel::tool::Access;
use miyu_session::testkit::{Play, Script};
use miyu_tool::testkit::{Act, Fake};
use support::Home;

/// 一件带显示名的假工具，像脚本、MCP 的：`act` 是跑起来做什么。
fn dice(act: Act) -> std::sync::Arc<dyn miyu_tool::Tool> {
    Fake::named(
        "roll",
        Access::Read,
        &[("en", "Dice"), ("zh", "掷骰子")],
        act,
    )
}

#[test]
fn a_running_step_takes_its_name_from_the_cores_tool_table() {
    // 核心 9-8 七补：还没有结果的那一步，名字照 `human.get` 的表，表里有这一刻写了显示名的工具。
    let script = Script::new([Play::Calls(vec![("roll".into(), "{}".into())])]);
    let home = Home::with_tool(script, "", dice(Act::Holds));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("掷一下");
    tui.wait_for("掷骰子");
    assert!(
        !tui.shows("roll"),
        "不写工具名：\n{}",
        tui.lines().join("\n")
    );
}

#[test]
fn a_finished_step_keeps_the_name_recorded_in_its_result_after_the_tool_is_gone() {
    // 核心 9-8 六补：跑完的照结果里记下的显示名（项目主人要「记当时的」）。工具卸了，表里没有它了，重开会话照日志补发，
    // 写得出「掷骰子」只能是照结果里记下的。
    let script = Script::new([
        // 一次调两下：段里两步，点开一步一行写名字（只有一步的点开直接铺开结果，不写标题）。
        Play::Calls(vec![
            ("roll".into(), "{}".into()),
            ("roll".into(), "{}".into()),
        ]),
        Play::Says("掷出了 4。"),
    ]);
    // 起来接着最近的会话：第二次起来就是重开这个会话（照日志补发）。
    let home = Home::with_tool(script, "[ui]\nstartup = \"recent\"\n", dice(Act::Echo));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("掷一下");
    tui.wait_for("掷出了 4。");
    opened_steps(&mut tui);
    drop(tui);
    home.restart_core_without_tools();
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("掷出了 4。");
    opened_steps(&mut tui);
    assert!(
        !tui.shows("roll"),
        "不写工具名：\n{}",
        tui.lines().join("\n")
    );
}

/// 她开口以后那一段收起成「Used 2 tools」：点一下铺开，等一步一行的标题写出显示名。
fn opened_steps(tui: &mut support::Tui) {
    tui.wait_for("Used 2 tools");
    let (y, line) = tui
        .lines()
        .into_iter()
        .enumerate()
        .find(|(_, l)| l.contains("Used 2 tools"))
        .expect("有收起的那一行");
    let x = line.find("Used").map_or(10, |b| line[..b].chars().count());
    tui.key(format!("\x1b[<0;{};{}M", x + 2, y + 1).as_bytes());
    tui.key(format!("\x1b[<0;{};{}m", x + 2, y + 1).as_bytes());
    tui.wait_for("掷骰子");
}
