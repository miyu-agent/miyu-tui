//! 伪终端里的端到端测试：待办接真的核心（蓝图 `tui.md`「后台命令、子代理和侧边栏」，核心 D-3）。她调 `todowrite`，
//! 核心推 `todos.changed`，待办那一块照它画。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn todowrite_shows_up_as_the_todo_list() {
    let args = serde_json::json!({"todos": [
        {"content": "读一遍代码", "status": "completed"},
        {"content": "改掉那个错", "status": "in_progress"},
        {"content": "跑测试", "status": "pending"}
    ]})
    .to_string();
    let script = Script::new([
        Play::Calls(vec![("todowrite".into(), args)]),
        Play::Says("列好了。"),
    ]);
    let home = Home::with_tools(script, "");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("排一下要做的");
    tui.wait_for("列好了。");
    tui.wait_for("待办 1/3");
    tui.wait_for("改掉那个错");
    tui.wait_for("跑测试");
    // 做完的排到末尾（2026-10-17 项目主人）：待办那一块在输入框上面，照最后出现的那一处找。
    let lines = tui.lines();
    let row = |word: &str| lines.iter().rposition(|l| l.contains(word));
    assert!(
        row("改掉那个错") < row("跑测试") && row("跑测试") < row("读一遍代码"),
        "{}",
        lines.join("\n")
    );
}

#[test]
fn coming_back_to_the_session_brings_the_todos_from_the_subscribe_reply() {
    // `todos.changed` 是瞬时的，补发里没有：回到会话时照订阅回应里的 `todos` 画（核心 D-3）。
    let args =
        serde_json::json!({"todos": [{"content": "跑测试", "status": "in_progress"}]}).to_string();
    let script = Script::new([
        Play::Calls(vec![("todowrite".into(), args)]),
        Play::Says("列好了。"),
    ]);
    let home = Home::with_tools(script, "[ui]\nstartup = \"recent\"\n");
    let mut first = home.tui("zh_CN.UTF-8");
    first.wait_for("工作区");
    first.say("排一下");
    first.wait_for("列好了。");
    drop(first);
    let mut again = home.tui("zh_CN.UTF-8");
    again.wait_for("列好了。");
    again.wait_for("待办 0/1");
    again.wait_for("跑测试");
}

#[test]
fn a_finished_list_goes_away_at_once() {
    // 2026-10-17 项目主人：全部完成之后直接消失，不全打勾留一会（原来 10-07 定的留 5 秒）。
    let start = serde_json::json!({"todos": [
        {"content": "读代码", "status": "in_progress"},
        {"content": "跑测试", "status": "pending"}
    ]})
    .to_string();
    let finish = serde_json::json!({"todos": [
        {"content": "读代码", "status": "completed"},
        {"content": "跑测试", "status": "completed"}
    ]})
    .to_string();
    let script = Script::new([
        Play::Calls(vec![("todowrite".into(), start)]),
        Play::Says("开始了。"),
        Play::Calls(vec![("todowrite".into(), finish)]),
        Play::Says("都做完了。"),
    ]);
    let home = Home::with_tools(script, "");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("开工");
    tui.wait_for("开始了。");
    tui.wait_for("待办 0/2");
    tui.say("继续");
    let end = std::time::Instant::now() + support::WAIT;
    while !tui.shows("都做完了。") {
        assert!(
            !tui.shows("待办 2/2"),
            "不全打勾露着：\n{}",
            tui.lines().join("\n")
        );
        assert!(std::time::Instant::now() < end, "等不到回答");
        tui.pump(std::time::Duration::from_millis(20));
    }
    tui.pump(std::time::Duration::from_millis(300));
    assert!(
        !tui.shows("待办 2/2") && !tui.shows("待办 0/2"),
        "当场收掉：\n{}",
        tui.lines().join("\n")
    );
}

#[test]
fn what_is_done_folds_into_the_bottom_line() {
    // 2026-10-17 项目主人：「已完成 N 项」挪到底部，不放顶部。
    let names = ["一号", "二号", "三号", "四号", "五号", "六号", "七号"];
    let todos: Vec<_> = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let status = match i {
                0..=3 => "completed",
                4 => "in_progress",
                _ => "pending",
            };
            serde_json::json!({"content": name, "status": status})
        })
        .collect();
    let args = serde_json::json!({ "todos": todos }).to_string();
    let script = Script::new([
        Play::Calls(vec![("todowrite".into(), args)]),
        Play::Says("列好了。"),
    ]);
    let home = Home::with_tools(script, "");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("排一下要做的");
    tui.wait_for("列好了。");
    tui.wait_for("已完成 4 项");
    let lines = tui.lines();
    let row = |word: &str| lines.iter().position(|l| l.contains(word));
    let (title, active, last, folded) = (
        row("待办 4/7"),
        row("五号"),
        row("七号"),
        row("已完成 4 项"),
    );
    assert!(
        title < active && active < last && last < folded && row("一号").is_none(),
        "没做完的在前，做完的收成最底下一行：\n{}",
        lines.join("\n")
    );
}
