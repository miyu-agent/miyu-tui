//! 伪终端里的端到端测试：侧边栏是挂载位（蓝图 `tui.md`「挂载位」第 2、4 条，反馈 1102 第 13 条）。配置文件里 `[[tui.side]]`
//! 写了的照写的先后、藏起来的不画，没写到的照出厂的先后接在后面。

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
    tui.wait_for("工作区");
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
