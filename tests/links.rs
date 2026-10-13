//! 正文里的链接（蓝图 `tui.md`「鼠标」）：悬停加下划线，指针挪开就去掉，挪到正文外面（左边的空白、输入框）也一样
//! （2026-10-13 验收 T-225：挪到左边的空白处，下划线和手形指针都留着）。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 鼠标挪到第 `row` 行第 `col` 列（从 0 数，SGR 报法）。
fn hover(tui: &mut support::Tui, row: u16, col: u16) {
    tui.send(format!("\x1b[<35;{};{}M", col + 1, row + 1).as_bytes());
    tui.pump(Duration::from_millis(400));
}

#[test]
fn a_link_loses_its_underline_when_the_pointer_leaves_the_body() {
    let home = Home::new(Script::new([Play::Says(
        "看这里：https://example.com/page 就行",
    )]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.say("给个链接");
    tui.wait_for("example.com/page");
    let lines = tui.lines();
    let (row, line) = lines
        .iter()
        .enumerate()
        .rev()
        .find(|(_, l)| l.contains("example.com/page"))
        .expect("链接画出来了");
    let row = u16::try_from(row).unwrap();
    // 照格子找「example」的列（宽字占两格）。
    let col = (0..200)
        .find(|&c| {
            tui.cell(row, c) == "e" && tui.cell(row, c + 1) == "x" && tui.cell(row, c + 2) == "a"
        })
        .unwrap_or_else(|| panic!("找得到 example：{line}"));
    hover(&mut tui, row, col);
    assert!(tui.underlined(row, col), "悬停加下划线");
    hover(&mut tui, row, 1);
    assert!(!tui.underlined(row, col), "挪到左边的空白处，下划线去掉");
}
