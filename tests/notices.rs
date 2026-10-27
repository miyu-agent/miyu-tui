//! 伪终端里的端到端测试：点了定时任务的通知切到那个会话（蓝图 `tui.md`「系统通知」第 8 条，核心 J-3 中）。kitty 点了
//! `a=focus,report` 的通知，往输入里回一句 `ESC ]99;i=<编号>;ESC \`：终端截下来切过去，不当按键打进输入框。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn a_clicked_kitty_notice_switches_to_its_session_and_types_nothing() {
    let script = Script::new([Play::Says("answer-one"), Play::Says("answer-two")]);
    let home = Home::with_settings(script, "");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("topic-one");
    tui.wait_for("answer-one");
    tui.say("/new");
    tui.say("topic-two");
    tui.wait_for("answer-two");
    // topic-one 那个会话的编号：日志里有这句话的那个目录。
    let sessions = home.root().join("home/alice/sessions");
    let first = std::fs::read_dir(&sessions)
        .expect("会话目录")
        .flatten()
        .find(|dir| {
            std::fs::read_dir(dir.path())
                .into_iter()
                .flatten()
                .flatten()
                .any(|f| std::fs::read_to_string(f.path()).is_ok_and(|t| t.contains("topic-one")))
        })
        .expect("topic-one 的会话")
        .file_name()
        .to_string_lossy()
        .into_owned();
    tui.key(format!("\x1b]99;i=miyu-s-{first};\x1b\\").as_bytes());
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
    let input = tui.input();
    assert!(
        !input.contains("99;") && !input.contains("miyu-s-"),
        "回话不当按键打进输入框：{input}"
    );
}
