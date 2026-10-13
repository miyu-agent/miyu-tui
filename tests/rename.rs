//! 伪终端里的端到端测试：`/rename` 改名，人起的标题写在输入框上边框右边（蓝图 `tui.md`「输入框」第 14 条、「改名」，
//! 2026-10-13 项目主人定）。从 `pty.rs` 分出来（那边过了 500 行）。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 输入框的上边框那一行（最后一行带 `╭` 的）。
fn input_border(tui: &support::Tui) -> String {
    tui.lines()
        .into_iter()
        .rev()
        .find(|l| l.contains('╭'))
        .unwrap_or_default()
}

#[test]
fn rename_sets_removes_and_refuses_a_too_long_title() {
    // 2026-10-01 项目主人定先做 `/rename`（蓝图「改名」）。
    let home = Home::new(Script::new([Play::Says("好。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("在吗");
    tui.wait_for("▣  ");
    tui.say("/rename   回文函数  ");
    tui.wait_for("已改名：回文函数");
    // 人改的标题写在输入框上边框右边（2026-10-13 项目主人定）；提示 2 秒后消失，再看边框。
    tui.pump(Duration::from_millis(2600));
    assert!(
        input_border(&tui).contains("回文函数"),
        "{}",
        tui.lines().join("\n")
    );
    tui.say(&format!("/rename {}", "长".repeat(201)));
    tui.wait_for("标题最多 200 个字");
    tui.pump(Duration::from_millis(500));
    tui.say("/rename");
    tui.wait_for("已清除标题");
    assert!(!input_border(&tui).contains("回文函数"), "去掉了就不写");
    // `/new` 以后、说第一句以前还没开会话。
    tui.pump(Duration::from_millis(500));
    tui.say("/new");
    tui.pump(Duration::from_millis(500));
    tui.say("/rename 早");
    tui.wait_for("会话尚未创建，无法重命名");
}

#[test]
fn a_title_a_person_gave_is_still_on_the_border_after_reopening() {
    // 接着老会话说话，人起的标题照样写在边框上（2026-10-13 项目主人定）：回放里有那次改名；改名落在没读的老页里的，
    // 照 `session.list` 的 `named`（核心 3-8 八补）。
    let home = Home::with_settings(
        Script::new([Play::Says("好。")]),
        "[ui]\nstartup = \"recent\"\n",
    );
    let mut first = home.tui("zh_CN.UTF-8");
    first.wait_for("工作区");
    first.say("在吗");
    first.wait_for("▣  ");
    first.say("/rename 回文函数");
    first.wait_for("已改名：回文函数");
    drop(first);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("好。");
    let end = std::time::Instant::now() + support::WAIT;
    while !input_border(&tui).contains("回文函数") && std::time::Instant::now() < end {
        tui.pump(Duration::from_millis(100));
    }
    assert!(
        input_border(&tui).contains("回文函数"),
        "{}",
        tui.lines().join("\n")
    );
}
