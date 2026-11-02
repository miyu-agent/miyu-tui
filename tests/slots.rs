//! 伪终端里的端到端测试：挂载位（蓝图 `tui.md`「挂载位」第 2、4 条，反馈 1102 第 13 条）。配置文件里 `[[tui.side]]`、
//! `[[tui.footer]]` 写了的照写的先后、藏起来的不画，没写到的照出厂的先后接在后面。

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

#[test]
fn the_status_line_follows_the_configured_order_and_hides_what_is_hidden() {
    // 底栏（SL-2）：左段藏起权限级别，右段把用量排到上下文前面。
    let script = Script::new([Play::Says("answer-one")]);
    let settings =
        "[[tui.footer]]\npart = \"level\"\nhidden = true\n\n[[tui.footer]]\npart = \"usage\"\n";
    let home = Home::with_settings(script, settings);
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    // 首页的底栏左段只有权限级别（模型要等开了会话），藏起来了就空着：等首页的提示。
    tui.wait_for("Tab 切换权限级别");
    tui.say("topic-one");
    tui.wait_for("answer-one");
    let end = std::time::Instant::now() + support::WAIT;
    loop {
        let lines = tui.lines();
        let footer = lines
            .iter()
            .rev()
            .find(|l| l.contains("deepseek-v4") && l.contains('Σ'));
        if let Some(footer) = footer {
            let total = footer.find('Σ').expect("有累计");
            let context = footer.rfind(" 110").expect("有上下文");
            assert!(total < context, "用量排到上下文前面：{footer}");
            assert!(!footer.contains("工作区"), "权限级别藏起来了：{footer}");
            break;
        }
        assert!(
            std::time::Instant::now() < end,
            "底栏要有模型和累计：\n{}",
            lines.join("\n")
        );
        tui.pump(std::time::Duration::from_millis(50));
    }
}
