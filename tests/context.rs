//! 伪终端里的端到端测试：`/context` 上下文框（蓝图 `tui.md`「上下文 `/context`」）和「无工具」预设（「新会话」第 3 条，
//! 核心 P-16 下）。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 等 `text` 从屏幕上消失。
fn until_gone(tui: &mut Tui, text: &str) {
    let end = Instant::now() + support::WAIT;
    while tui.shows(text) {
        assert!(
            Instant::now() < end,
            "「{text}」还在：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(50));
    }
}

#[test]
fn context_lists_the_parts_and_the_usage_by_purpose_once_the_session_is_open() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    // 还没开会话：底栏照新会话的预览写用了多少（核心 CX-1 下），`/context` 是预览、没有累计。
    let end = Instant::now() + support::WAIT;
    // 「工作区」那一行右边是预览的数（测试核心工具少，不到 1k）。
    let previewed = |tui: &Tui| {
        tui.lines().iter().any(|l| {
            l.contains("工作区")
                && l.trim_end()
                    .ends_with(|c: char| c.is_ascii_digit() || c == ')')
        })
    };
    while !previewed(&tui) {
        assert!(
            Instant::now() < end,
            "底栏没写预览：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(50));
    }
    tui.say("/context");
    tui.wait_for("核心说明");
    assert!(
        tui.shows("估算") && !tui.shows("本会话累计"),
        "{}",
        tui.lines().join("\n")
    );
    tui.key(b"\x1b");
    until_gone(&mut tui, "核心说明");
    tui.say("在吗");
    tui.wait_for("好。");
    tui.say("/context");
    tui.wait_for("本会话累计");
    let screen = tui.lines().join("\n");
    for text in ["已用", "系统提示", "对话", "主对话", "合计", "次数"] {
        assert!(screen.contains(text), "少了「{text}」：\n{screen}");
    }
    tui.key(b"\x1b");
    until_gone(&mut tui, "本会话累计");
}

#[test]
fn a_session_opened_with_no_tools_says_so_and_has_no_tools() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    // `-` 是空的编号：无工具（同无人格）。
    let mut tui = home.tui_with("zh_CN.UTF-8", &[("MIYU_TUI_PRESET", "-")]);
    tui.wait_for("工作区");
    tui.say("在吗");
    tui.wait_for("好。");
    // 测具的终端窄、没有侧边栏：看 `/status` 的预设那一行。
    tui.say("/status");
    tui.wait_for("会话 ID");
    let row = tui
        .lines()
        .into_iter()
        .find(|l| l.contains("预设"))
        .unwrap_or_default();
    assert!(row.contains("无工具"), "{}", tui.lines().join("\n"));
    tui.key(b"\x1b");
    until_gone(&mut tui, "会话 ID");
    tui.say("/tool-list");
    tui.wait_for("没有能用的工具");
}
