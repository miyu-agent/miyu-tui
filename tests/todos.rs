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
    // 收着时做完的不露（2026-10-26 项目主人，第 4a 条）：待办那一块在输入框上面，照最后出现的那一处找。
    let lines = tui.lines();
    let row = |word: &str| lines.iter().rposition(|l| l.contains(word));
    let title = row("待办 1/3").expect("有标题");
    assert!(
        row("读一遍代码").is_none_or(|r| r < title),
        "收着时做完的不露：\n{}",
        lines.join("\n")
    );
    // 点标题展开：做完的排到末尾（2026-10-17 项目主人）。
    let col = lines[title]
        .find("待办")
        .map_or(0, |b| lines[title][..b].chars().count())
        + 1;
    tui.key(format!("\x1b[<0;{col};{}M\x1b[<0;{col};{}m", title + 1, title + 1).as_bytes());
    tui.wait_for("读一遍代码");
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
fn folded_it_shows_what_is_in_progress_and_the_next_two() {
    // 2026-10-26 项目主人（反馈第 1 条）：只露进行中的一条、待进行的两条，标题写总数，底下不再接「还有」「已完成」。
    let names = [
        "一号", "二号", "三号", "四号", "五号", "六号", "七号", "八号",
    ];
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
    tui.wait_for("待办 4/8");
    tui.wait_for("七号");
    let lines = tui.lines();
    let row = |word: &str| lines.iter().position(|l| l.contains(word));
    assert!(
        row("待办 4/8") < row("五号") && row("五号") < row("六号") && row("六号") < row("七号"),
        "{}",
        lines.join("\n")
    );
    assert!(
        row("八号").is_none()
            && row("一号").is_none()
            && row("已完成").is_none()
            && row("还有").is_none(),
        "只露三项：\n{}",
        lines.join("\n")
    );
}

#[test]
fn todo_clear_folds_the_list_at_once() {
    // 核心 D-3 再补：`/todo clear` 清掉待办，推来空清单，这一块当场收（2026-10-17 项目主人：全做完、清掉都直接消失）。
    let args = serde_json::json!({"todos": [
        {"content": "读代码", "status": "in_progress"},
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
    tui.say("排一下");
    tui.wait_for("列好了。");
    tui.wait_for("待办 0/2");
    tui.say("/todo clear");
    // 回执照提示画，末尾的句号去掉。
    tui.wait_for("已清空待办");
    tui.pump(std::time::Duration::from_millis(300));
    assert!(!tui.shows("待办 0/2"), "{}", tui.lines().join("\n"));
}

#[test]
fn with_a_sidebar_and_its_mascot_the_items_still_show_above_the_input() {
    // 2026-10-27 用户报（测试包 1，Arch、foot）：让她写待办，只看得到「待办 1/3」、看不到每一项，有时整块都没有，重开才有。
    // 窗口宽、有侧边栏时待办原来在侧边栏最底下，吉祥物、会话那几段占满了高，剩不下几行。待办挪到输入框上面（第 4a 条）以后
    // 不受侧边栏多高管。
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
    let home = Home::with_tools(script, "[tui]\nmascot_home = true\nmascot_sidebar = true\n");
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    tui.wait_for("工作区");
    tui.say("排一下要做的");
    tui.wait_for("列好了。");
    tui.wait_for("待办 1/3");
    tui.wait_for("跑测试");
    let lines = tui.lines();
    let main = |word: &str| {
        lines
            .iter()
            .any(|l| l.find(word).is_some_and(|at| at < 100))
    };
    assert!(
        main("待办 1/3") && main("改掉那个错") && main("跑测试"),
        "每一项都在主列、输入框上面：\n{}",
        lines.join("\n")
    );
}
