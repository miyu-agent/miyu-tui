//! 伪终端里的端到端测试：侧边栏比窗口高的时候滚轮滚得动，滚下去点得准（发版前 check 第 33 条）。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 侧边栏里（窗口 140 列宽，侧边栏在右边）含 `word` 的那一行。照显示宽度认：左边正文里有中文的一行，字数比列数少。
fn side(lines: &[String], word: &str) -> Option<usize> {
    lines.iter().position(|l| {
        l.find(word)
            .is_some_and(|at| unicode_width::UnicodeWidthStr::width(&l[..at]) > 100)
    })
}

/// 在侧边栏上滚一格滚轮（`down` 往下）。
fn wheel(tui: &mut Tui, down: bool) {
    let button = if down { 65 } else { 64 };
    tui.key(format!("\x1b[<{button};130;20M").as_bytes());
    tui.pump(Duration::from_millis(80));
}

#[test]
fn a_tall_sidebar_scrolls_and_shows_every_jump() {
    const TURNS: usize = 30;
    let plays: Vec<Play> = (1..=TURNS)
        .map(|i| Play::Says(Box::leak(format!("回答 {i}。").into_boxed_str())))
        .collect();
    let home = Home::with_settings(Script::new(plays), "[tui]\nsidebar_jumps = 100\n");
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    tui.wait_for("开放权限");
    for i in 1..=TURNS {
        tui.say(&format!("问{i:02}号"));
        tui.wait_for(&format!("回答 {i}。"));
    }
    tui.pump(Duration::from_millis(300));
    let lines = tui.lines();
    assert!(
        side(&lines, "问01号").is_some() && side(&lines, "未命名会话").is_some(),
        "最早一句在顶上：\n{}",
        lines.join("\n")
    );
    for _ in 0..30 {
        wheel(&mut tui, true);
    }
    let lines = tui.lines();
    assert!(
        side(&lines, "问30号").is_some() && side(&lines, "未命名会话").is_none(),
        "滚下去看得到最新一句：\n{}",
        lines.join("\n")
    );
    // 指针停在侧边栏上：最右边那一列画滚动条，滚到底了滑块贴底；指针在正文上不画（同正文的）。
    let column = |tui: &Tui| -> Vec<String> {
        let rows = u16::try_from(tui.lines().len()).unwrap();
        (0..rows).map(|r| tui.cell(r, 139)).collect()
    };
    tui.key(b"\x1b[<35;30;20M");
    tui.pump(Duration::from_millis(200));
    let away = column(&tui);
    assert!(
        away.iter().all(|c| c.trim().is_empty()),
        "指针不在侧边栏上不画：{away:?}"
    );
    tui.key(b"\x1b[<35;130;20M");
    tui.pump(Duration::from_millis(200));
    let bar = column(&tui);
    assert!(
        bar.iter().any(|c| c == "│") && bar.last().is_some_and(|c| c == "█"),
        "有滚动条、滑块贴底：{bar:?}"
    );
    // 滚过以后点侧边栏里的一句：照滚过的换算，跳到的是点的那一句（滚过的几行算错了会跳到别的那句）。
    let row = side(&lines, "问05号").expect("列着");
    let col = unicode_width::UnicodeWidthStr::width(
        &lines[row][..lines[row].find("问05号").expect("在")],
    ) + 1;
    tui.key(format!("\x1b[<0;{col};{0}M\x1b[<0;{col};{0}m", row + 1).as_bytes());
    let start = std::time::Instant::now();
    loop {
        tui.pump(Duration::from_millis(20));
        let now = tui.lines();
        if now
            .iter()
            .position(|l| l.contains("┃ 问05号"))
            .is_some_and(|at| at <= 2)
        {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "没跳到点的那一句：\n{}",
            now.join("\n")
        );
    }
}

#[test]
fn an_old_session_lists_every_jump() {
    const TURNS: usize = 25;
    let plays: Vec<Play> = (1..=TURNS + 3)
        .map(|i| Play::Says(Box::leak(format!("回答 {i}。").into_boxed_str())))
        .collect();
    let home = Home::with_settings(
        Script::new(plays),
        "[ui]\nstartup = \"recent\"\n[tui]\nsidebar_jumps = 100\n",
    );
    let mut first = home.tui_wide("zh_CN.UTF-8", 140);
    first.wait_for("开放权限");
    for i in 1..=TURNS {
        first.say(&format!("问{i:02}号"));
        first.wait_for(&format!("回答 {i}。"));
    }
    drop(first);
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    tui.wait_for(&format!("回答 {TURNS}。"));
    tui.pump(Duration::from_millis(500));
    for i in TURNS + 1..=TURNS + 3 {
        tui.say(&format!("问{i:02}号"));
        tui.wait_for(&format!("回答 {i}。"));
    }
    // 只读了最新一页（核心默认 20 轮）：侧边栏要列 100 条，接着读更早的，最早一句也列上。
    let start = std::time::Instant::now();
    while side(&tui.lines(), "问01号").is_none() {
        assert!(
            start.elapsed() < support::WAIT,
            "更早的没读上来：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(50));
    }
}
