//! 伪终端里的端到端测试：人打的 `/dream`（蓝图 `tui.md`「整理记忆」，核心 R-21）接真的核心。
//!
//! 测试核心的剧本排了一次性入口的答案（`Script::once`，核心 f94ac37a）才整理得起来；没排的 `/dream` 被拒
//! （`memory_unavailable`）。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 被拒：弹核心的原话，那一块收掉、不留在正文末尾（第 1 条）。
#[test]
fn a_refused_dream_says_why_and_leaves_no_block() {
    let home = Home::new(Script::new([Play::Says("好的。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("你好");
    tui.wait_for("好的。");
    tui.say("/dream");
    tui.wait_for("记忆不可用");
    for _ in 0..6 {
        tui.pump(Duration::from_millis(100));
        assert!(!tui.shows("✦ 整理记忆"), "{}", tui.lines().join("\n"));
    }
}

/// 首页打的 `/dream`（2026-10-20 项目主人验收报：只在后台整理，什么都不画）：当场换到正文画那一块，浮过这一批交的记忆，
/// 完了收成暗色 `✦` 打头的回执，点开列日志里真正存下的删、摘要（第 1、4、6 条）。
#[test]
fn dream_from_the_home_page_shows_the_block_and_a_receipt_that_opens() {
    let script = Script::new([Play::Says("好的。")]).once([
        r#"{"retired": [{"id": "m1", "kind": "outdated", "by": "m2", "why": "搬家了"}], "summary": "用户住在杭州。"}"#,
        r#"{"keywords": []}"#,
    ]);
    let home = Home::with_settings(script.clone(), "[persona]\ndefault = \"pirate\"\n");
    home.persona("pirate", "[persona]\nname = \"海盗\"\n", None);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("你好");
    tui.wait_for("好的。");
    tui.say("/remember 用户住在上海");
    tui.wait_for("记下了：用户住在上海");
    tui.say("/remember 用户搬到杭州了");
    tui.wait_for("记下了：用户搬到杭州了");
    tui.say("/new");
    tui.pump(Duration::from_millis(500));
    tui.say("/dream");
    tui.wait_for("✦ 整理记忆");
    // 她写的删推到滚动区：原文照编号取、只写种类。
    tui.wait_for("✗ 用户住在上海 · 过时");
    tui.wait_for("(・ω・)");
    tui.wait_for("✦ 整理完成：检查 2 条");
    let end = std::time::Instant::now() + support::WAIT;
    while tui.shows("✦ 整理记忆") {
        assert!(
            std::time::Instant::now() < end,
            "{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(50));
    }
    // 点开回执：列日志里真正存下的删和摘要全文。
    let row = tui
        .lines()
        .iter()
        .position(|l| l.contains("✦ 整理完成"))
        .expect("回执那一行");
    let col = tui.lines()[row]
        .find('✦')
        .map_or(10, |at| tui.lines()[row][..at].chars().count() + 3);
    tui.key(format!("\x1b[<0;{col};{}M\x1b[<0;{col};{}m", row + 1, row + 1).as_bytes());
    tui.wait_for("✦ 摘要：用户住在杭州。");
    tui.wait_for("✗ 用户住在上海 · 过时");
}

/// 没在整理时打 `/dream stop`（核心 R-22）：交给核心，弹它的原话；不当成又一趟整理。
#[test]
fn dream_stop_without_a_dream_says_there_is_none() {
    let home = Home::new(Script::new([Play::Says("好的。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("你好");
    tui.wait_for("好的。");
    tui.say("/dream stop");
    tui.wait_for("没有正在进行的整理");
    assert!(!tui.shows("✦ 整理记忆"), "{}", tui.lines().join("\n"));
}
