//! 伪终端里的端到端测试：`/jump` 列出这个会话里说过的话，选一句跳过去，那一句顶到视口最上面（蓝图 `tui.md`「跳到说过的话」，
//! 2026-10-16 项目主人要的）。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 一段回答：每段一行，`tag` 打头好认。比一屏短：长过视口的回答会停在开头（「正文」第 1 条），最后一段露不出来，流得慢的
/// 机器上等不到它（2026-10-16 macOS CI）；三轮加起来照样把第二问挤出视口。
fn long(tag: &str) -> &'static str {
    let text: Vec<String> = (1..=10).map(|i| format!("{tag} 第 {i} 段。")).collect();
    Box::leak(text.join("\n\n").into_boxed_str())
}

#[test]
fn jump_lists_what_was_said_and_puts_the_picked_one_at_the_top() {
    let home = Home::new(Script::new([
        Play::Says(long("甲")),
        Play::Says(long("乙")),
        Play::Says(long("丙")),
    ]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    for (said, last) in [
        ("第一问 alpha", "甲 第 10 段。"),
        ("第二问 beta", "乙 第 10 段。"),
        ("第三问 gamma", "丙 第 10 段。"),
    ] {
        tui.say(said);
        tui.wait_for(last);
    }
    tui.pump(Duration::from_millis(500));
    assert!(!tui.shows("第二问 beta"), "第二问早滚出视口了");
    tui.say("/jump");
    tui.wait_for("跳到  3 条");
    for said in ["第一问 alpha", "第二问 beta", "第三问 gamma"] {
        assert!(
            tui.shows(said),
            "列表里有「{said}」：\n{}",
            tui.lines().join("\n")
        );
    }
    tui.type_text("beta");
    tui.pump(Duration::from_millis(300));
    tui.key(b"\r");
    // 一画出来就取颜色（竖线只亮 1 秒：原来固定等 0.5 秒再取，macOS 的 CI 慢时取到的已经是亮过以后的颜色）。
    let start = std::time::Instant::now();
    let at = loop {
        tui.pump(Duration::from_millis(20));
        let lines = tui.lines();
        let found = lines.iter().position(|l| l.contains("第二问 beta"));
        if let Some(at) = found.filter(|&at| at <= 2 && !tui.shows("跳到")) {
            break at;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "没跳过去、没顶到视口最上面（竖线那一行之下）、列表没关：\n{}",
            lines.join("\n")
        );
    };
    let lines = tui.lines();
    // 竖线亮一下（第 3 条）：刚跳过去和过了一秒多颜色不一样。
    let col = lines[at].chars().take_while(|&c| c != '┃').count();
    let (row, col) = (u16::try_from(at).unwrap(), u16::try_from(col).unwrap());
    assert_eq!(tui.cell(row, col), "┃");
    let lit = tui.foreground(row, col);
    let start = std::time::Instant::now();
    while tui.foreground(row, col) == lit && start.elapsed() < Duration::from_secs(5) {
        tui.pump(Duration::from_millis(100));
    }
    assert_ne!(tui.foreground(row, col), lit, "亮过以后画回原来的颜色");
}

#[test]
fn jump_reads_the_earlier_pages_of_an_old_session_without_scrolling_up() {
    // 比一页（核心默认 20 轮）多两轮；重起以后进最近的那个会话，只读了最新一页（`tests/pages.rs`）。
    const TURNS: usize = 22;
    let plays: Vec<Play> = (1..=TURNS)
        .map(|i| Play::Says(Box::leak(format!("第 {i} 号的回答。").into_boxed_str())))
        .collect();
    let home = Home::with_settings(Script::new(plays), "[ui]\nstartup = \"recent\"\n");
    let mut first = home.tui("zh_CN.UTF-8");
    first.wait_for("工作区");
    for i in 1..=TURNS {
        first.say(&format!("第 {i} 句"));
        first.wait_for(&format!("第 {i} 号的回答。"));
        first.pump(Duration::from_millis(100));
    }
    drop(first);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("第 22 号的回答。");
    tui.pump(Duration::from_millis(300));
    tui.say("/jump");
    // 更早的那一页不等滚到顶就读上来：22 句都在。
    tui.wait_for("跳到  22 条");
    tui.type_text("第 1 句");
    tui.pump(Duration::from_millis(300));
    tui.key(b"\r");
    tui.pump(Duration::from_millis(500));
    let lines = tui.lines();
    let at = lines
        .iter()
        .position(|l| l.contains("┃ 第 1 句"))
        .unwrap_or_else(|| panic!("没跳到第 1 句：\n{}", lines.join("\n")));
    assert!(at <= 2, "第 1 句顶到最上面：\n{}", lines.join("\n"));
}
