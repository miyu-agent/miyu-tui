//! 伪终端里的端到端测试：人打的 `/dream`（蓝图 `tui.md`「整理记忆」，核心 R-21）接真的核心。
//!
//! 测试核心的脚本模型没有一次性入口，核心不交整理要的几样，`/dream` 一律被拒（`memory_unavailable`）：这里测被拒的那条路；
//! 整理时画什么由单元测试（`src/dream`、`src/ui/dream_rows`）和真核心的自测守着。

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
