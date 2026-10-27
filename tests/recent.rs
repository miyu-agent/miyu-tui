//! 伪终端里的端到端测试：侧边栏「最近会话」（蓝图 `tui.md`「后台命令、子代理和侧边栏」第 7 条，2026-10-26 项目主人，反馈
//! 第 21 条）。照会话列表的先后，当前这个在原位、行首圆点；点一行切过去，切过去的不挪到最上面。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn the_sidebar_lists_recent_sessions_and_a_click_switches() {
    let script = Script::new([Play::Says("answer-one"), Play::Says("answer-two")]);
    let home = Home::with_settings(script, "");
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    tui.wait_for("工作区");
    tui.say("topic-one");
    tui.wait_for("answer-one");
    tui.say("/new");
    tui.say("topic-two");
    tui.wait_for("answer-two");
    tui.wait_for("最近会话");
    // 没标题的照预览：新的（topic-two，也是当前这个）在上面，topic-one 在下面。
    let side = |lines: &[String], word: &str| {
        lines
            .iter()
            .position(|l| l.find(word).is_some_and(|at| at > 100))
    };
    let lines = tui.lines();
    let (title, two, one) = (
        lines.iter().position(|l| l.contains("最近会话")),
        side(&lines, "topic-two"),
        side(&lines, "topic-one"),
    );
    assert!(
        title < two && two < one,
        "新的在上面：\n{}",
        lines.join("\n")
    );
    // 记号挂在缩进那两格里，名字和别的几段的内容对齐（2026-10-27 项目主人：缩进有待优化）。
    let col = |row: usize, word: &str| {
        lines[row][..lines[row].find(word).expect("在")]
            .chars()
            .count()
    };
    let usage = side(&lines, "共 ").expect("有用量那一段");
    assert_eq!(
        col(one.expect("列着"), "topic-one"),
        col(usage, "共 "),
        "\n{}",
        lines.join("\n")
    );
    // 点 topic-one 那一行：切过去，正文换成它的。
    let row = one.expect("列着 topic-one");
    let col = lines[row].find("topic-one").expect("在") + 1;
    let at = format!("\x1b[<0;{col};{}M\x1b[<0;{col};{}m", row + 1, row + 1);
    tui.key(at.as_bytes());
    tui.wait_for("answer-one");
    let end = std::time::Instant::now() + support::WAIT;
    while tui.shows("answer-two") {
        assert!(
            std::time::Instant::now() < end,
            "没切过去：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(std::time::Duration::from_millis(50));
    }
    // 切过去的不顶到最上面（2026-10-27 项目主人：「正常按照时间顺序就好了。否则很难找」）：照旧 topic-two 在上，圆点挪到
    // topic-one 那一行。
    let end = std::time::Instant::now() + support::WAIT;
    loop {
        let lines = tui.lines();
        let (two, one) = (side(&lines, "topic-two"), side(&lines, "topic-one"));
        let marked = one.is_some_and(|row| lines[row].contains("● topic-one"));
        if marked {
            assert!(two < one, "切过去的不挪：\n{}", lines.join("\n"));
            break;
        }
        assert!(
            std::time::Instant::now() < end,
            "圆点没挪到切过去的那一行：\n{}",
            lines.join("\n")
        );
        tui.pump(std::time::Duration::from_millis(50));
    }
}
