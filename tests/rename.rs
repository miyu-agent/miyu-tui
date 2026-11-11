//! 伪终端里的端到端测试：`/rename` 改名、只打 `/rename` 让模型重起（核心 TT-1），人起的标题写在输入框上边框右边（蓝图 `tui.md`「输入框」第 14 条、「改名」，
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
fn rename_sets_and_refuses_a_too_long_title() {
    // 2026-10-01 项目主人定先做 `/rename`（蓝图「改名」）。
    let home = Home::new(Script::new([Play::Says("好。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("在吗");
    tui.wait_for_done();
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
    first.wait_for("开放权限");
    first.say("在吗");
    first.wait_for_done();
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

#[test]
fn a_bare_rename_has_the_model_title_the_session_again() {
    // 2026-11-11 项目主人：「直接运行 rename 应该是用一次 ai 请求去更新标题而不是清除标题」。核心 TT-1 的 `session.retitle`。
    // 起标题的请求照先后回：第一轮答完内核自己起的那一次、`/rename` 要的那一次。
    let script = Script::new([Play::Says("好。")])
        .titles([Play::Says("自动起的"), Play::Says("重起的标题")]);
    let home = Home::new(script);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("帮我写个回文函数");
    tui.wait_for_done();
    // 人先起一个名：写在边框上。
    tui.say("/rename 人起的");
    tui.wait_for("已改名：人起的");
    tui.pump(Duration::from_millis(2600));
    assert!(input_border(&tui).contains("人起的"));
    tui.say("/rename");
    tui.wait_for("正在重新起标题");
    // 起好了标题照推送换：不再是人起的，边框上不写了；会话列表里是模型起的那个。
    let end = std::time::Instant::now() + support::WAIT;
    while input_border(&tui).contains("人起的") && std::time::Instant::now() < end {
        tui.pump(Duration::from_millis(100));
    }
    assert!(
        !input_border(&tui).contains("人起的"),
        "{}",
        tui.lines().join("\n")
    );
    tui.pump(Duration::from_millis(2600));
    tui.say("/sessions");
    tui.wait_for("重起的标题");
}
