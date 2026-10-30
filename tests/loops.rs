//! 伪终端里的端到端测试：这个会话挂着循环时，输入框上面写下一次几点，侧边栏多一段「循环」（蓝图 `tui.md`「循环」，
//! 2026-10-30 项目主人：「我不知道当前 loop 的状态」）。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn a_loop_shows_above_the_input_and_in_the_sidebar() {
    // 先说一句开会话；`/loop` 收下以后马上先跑一次，剧本再答一句。
    let home = Home::with_tools(
        Script::new([Play::Says("在。"), Play::Says("CI 是绿的。")]),
        "",
    );
    // 宽到画侧边栏（`layout.json` 的 `sidebar_from`）。
    let mut tui = home.tui_wide("zh_CN.UTF-8", 160);
    tui.wait_for("工作区");
    tui.say("在吗");
    tui.wait_for("在。");
    tui.say("/loop 10m 看一眼 CI");
    tui.wait_for("↻ ");
    tui.wait_for("每 10 分钟 · 下次");
    assert!(tui.shows("看一眼 CI"), "{}", tui.lines().join("\n"));
    // 停掉：那一段和那一截都不画了。
    tui.say("/loop off");
    let end = std::time::Instant::now() + support::WAIT;
    while tui.shows("每 10 分钟") || tui.shows("↻ ") {
        assert!(
            std::time::Instant::now() < end,
            "停了还在：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(std::time::Duration::from_millis(100));
    }
}
