//! 伪终端里的端到端测试：挂载位（蓝图 `tui.md`「挂载位」第 2、4 条，反馈 1102 第 13 条）。配置文件里 `[[tui.side]]`、
//! `[[tui.footer]]`、`[[tui.above]]` 写了的照写的先后、藏起来的不画，没写到的照出厂的先后接在后面。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 侧边栏那几列（窗口 140 列宽，侧边栏在右边）里有 `word` 的那一行。
fn side(lines: &[String], word: &str) -> Option<usize> {
    lines
        .iter()
        .position(|l| l.find(word).is_some_and(|at| l[..at].chars().count() > 100))
}

#[test]
fn the_sidebar_follows_the_configured_order_and_hides_what_is_hidden() {
    let script = Script::new([Play::Says("answer-one")]);
    let settings =
        "[[tui.side]]\npart = \"recent\"\n\n[[tui.side]]\npart = \"cwd\"\nhidden = true\n";
    let home = Home::with_settings(script, settings);
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    tui.wait_for("开放权限");
    tui.say("topic-one");
    tui.wait_for("answer-one");
    tui.wait_for("最近会话");
    let end = std::time::Instant::now() + support::WAIT;
    loop {
        let lines = tui.lines();
        let (recent, usage) = (side(&lines, "最近会话"), side(&lines, "用量"));
        if recent.is_some() && usage.is_some() {
            assert!(
                recent < usage,
                "写了的排在前面，没写到的照出厂接后面：\n{}",
                lines.join("\n")
            );
            assert!(
                side(&lines, "工作区").is_none(),
                "藏起来的不画：\n{}",
                lines.join("\n")
            );
            break;
        }
        assert!(
            std::time::Instant::now() < end,
            "侧边栏要有最近会话和用量：\n{}",
            lines.join("\n")
        );
        tui.pump(std::time::Duration::from_millis(50));
    }
}

/// 底栏那一行里用量那一格（环 + 累计）的那一行：模型名和环都在。
fn footer_line(lines: &[String]) -> Option<String> {
    lines
        .iter()
        .rev()
        .find(|l| l.contains("deepseek-v4") && l.contains(['○', '◔', '◑', '◕', '●']))
        .cloned()
}

#[test]
fn the_status_line_follows_the_configured_order_and_hides_what_is_hidden() {
    // 底栏（SL-2）：左段藏起权限级别，右段只有用量那一格（排期第 22 条：环 + 累计 + 花费），照常写。
    let script = Script::new([Play::Says("answer-one")]);
    let settings = "[[tui.footer]]\npart = \"level\"\nhidden = true\n";
    let home = Home::with_settings(script, settings);
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    // 首页的底栏左段只有权限级别（模型要等开了会话），藏起来了就空着：等首页的提示。
    tui.wait_for("Tab 切换权限级别");
    tui.say("topic-one");
    tui.wait_for("answer-one");
    let end = std::time::Instant::now() + support::WAIT;
    loop {
        if let Some(footer) = footer_line(&tui.lines()) {
            assert!(!footer.contains("开放权限"), "权限级别藏起来了：{footer}");
            assert!(
                !footer.contains('Σ') && !footer.contains("(C"),
                "不再写 Σ、命中率：{footer}"
            );
            break;
        }
        assert!(
            std::time::Instant::now() < end,
            "底栏要有模型和用量那一格：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(std::time::Duration::from_millis(50));
    }
    // 点用量那一格开 `/context`（「框下面那一行」用量那一格）。
    let lines = tui.lines();
    let footer = footer_line(&lines).expect("在");
    let row = lines.iter().rposition(|l| *l == footer).expect("在");
    let ring = footer.find(['○', '◔', '◑', '◕', '●']).expect("有环");
    let col = footer[..ring].chars().count() + 1;
    tui.key(format!("\x1b[<0;{col};{0}M\x1b[<0;{col};{0}m", row + 1).as_bytes());
    tui.wait_for("上下文明细");
}

#[test]
fn a_hidden_usage_part_leaves_the_right_side_empty() {
    let script = Script::new([Play::Says("answer-one")]);
    let home = Home::with_settings(script, "[[tui.footer]]\npart = \"usage\"\nhidden = true\n");
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    tui.wait_for("开放权限");
    tui.say("topic-one");
    tui.wait_for("answer-one");
    tui.pump(std::time::Duration::from_millis(500));
    assert!(
        footer_line(&tui.lines()).is_none(),
        "藏起来的不画：\n{}",
        tui.lines().join("\n")
    );
}

#[test]
fn a_hidden_todo_list_does_not_show_above_the_input_box() {
    // 输入框上面那一块（SL-2）：藏起待办，她写了待办也不画。
    let args = serde_json::json!({"todos": [
        {"content": "改掉那个错", "status": "in_progress"},
        {"content": "跑测试", "status": "pending"}
    ]})
    .to_string();
    let script = Script::new([
        Play::Calls(vec![("todowrite".into(), args)]),
        Play::Says("列好了。"),
    ]);
    let home = Home::with_tools(script, "[[tui.above]]\npart = \"todo\"\nhidden = true\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("排一下要做的");
    tui.wait_for("列好了。");
    tui.pump(std::time::Duration::from_millis(500));
    assert!(
        !tui.shows("待办 0/2") && !tui.shows("跑测试"),
        "藏起来的不画：\n{}",
        tui.lines().join("\n")
    );
}

#[test]
fn the_wheel_scrolls_a_sidebar_taller_than_the_window() {
    // 排期第 14 条：侧边栏比窗口高时，滚轮在侧边栏上像正文那样滚。说过一句以后会话、工作区、上下文、用量、最近会话几段，
    // 窗口矮到 14 行时最近会话排不下；在侧边栏上往下滚，它露出来，上面几段跟着往上走。
    let script = Script::new([Play::Says("answer-one")]);
    let home = Home::with_settings(script, "");
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    tui.wait_for("开放权限");
    tui.say("topic-one");
    tui.wait_for("answer-one");
    tui.wait_for("最近会话");
    tui.resize(14, 140);
    let end = std::time::Instant::now() + support::WAIT;
    while side(&tui.lines(), "最近会话").is_some() || side(&tui.lines(), "工作区").is_none()
    {
        assert!(
            std::time::Instant::now() < end,
            "矮了排不下最近会话：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(std::time::Duration::from_millis(50));
    }
    let before = side(&tui.lines(), "工作区").expect("在");
    // 滚轮在侧边栏上（第 120 列、第 5 行）往下滚几格。
    for _ in 0..4 {
        tui.key(b"\x1b[<65;120;5M");
    }
    let end = std::time::Instant::now() + support::WAIT;
    while side(&tui.lines(), "最近会话").is_none() {
        assert!(
            std::time::Instant::now() < end,
            "滚下去露出最近会话：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(std::time::Duration::from_millis(50));
    }
    // 上面几段跟着往上走（滚到底就停，这里只比窗口高几行，工作区那一段还在）。
    let after = side(&tui.lines(), "工作区");
    assert!(
        after.is_none_or(|row| row < before),
        "上面几段跟着往上走：\n{}",
        tui.lines().join("\n")
    );
}
