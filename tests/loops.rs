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
    tui.wait_for("开放权限");
    tui.say("在吗");
    tui.wait_for("在。");
    tui.say("/loop 10m 看一眼 CI");
    // 记号照图标那一套，出厂 nerd 是钟（2026-11-02 项目主人）。
    tui.wait_for("\u{f017} ");
    // 交进来的那一句画成一行，打头的钟也照这一套（ACC-1121-#2：原来写死 `plain` 的 `◷`）。
    tui.wait_for("\u{f017} 循环 · 继续");
    // 要做的那几个字和「每 10 分钟」不一定一起读到：等两样都在了再看。
    tui.wait_for_all(&["每 10 分钟 · 下次", "看一眼 CI"]);
    assert!(tui.shows("看一眼 CI"), "{}", tui.lines().join("\n"));
    // 停掉：那一段和那一截都不画了。
    tui.say("/loop off");
    let end = std::time::Instant::now() + support::WAIT;
    // 输入框上面那一截没了；交进来过的那一行「循环 · 继续」留在正文里。
    let tag = |l: &String| l.contains("\u{f017} ") && !l.contains("循环 · 继续");
    while tui.shows("每 10 分钟") || tui.lines().iter().any(tag) {
        assert!(
            std::time::Instant::now() < end,
            "停了还在：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(std::time::Duration::from_millis(100));
    }
}

#[test]
fn a_loop_round_held_by_a_restorable_undo_says_so() {
    // ACC-1121-#1（2026-11-20 主人本机）：撤销了一轮、还能恢复时，核心只记下循环这一轮、不开（`last.result` 是 `held`），
    // 界面上什么都没有，看着像循环没了。现在底下提示一句。
    let home = Home::with_tools(Script::new([Play::Says("在。")]), "");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("在吗");
    tui.wait_for("在。");
    tui.say("/undo");
    tui.wait_for("已撤销");
    tui.say("/loop 10m 看一眼 CI");
    tui.wait_for("循环这一轮没开始");
}
