//! 伪终端里的端到端测试：前缀要变时问人（蓝图「前缀要变」，排期第 59 条；核心 sessions.md「前缀要变」）。终端握手声明
//! `caps.prefix_ask`；改了这个会话前缀里的一份覆盖以后，下一句先弹框列出那个文件：「直接发送」照发、她接着答；「取消」那句
//! 不发、字回到框里；再发一次照样问，选「压缩上下文并发送」压不了的弹拒绝的话、字回到框里。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 改的那一份覆盖（同核心 `tests/prefix_ask.rs`）。
const RULE: &str = "core/local-paths-rule.txt";

/// 改那一份覆盖，等核心看到：界面这头不知道核心什么时候看到，照改过以后多等一会儿。
fn edit(home: &Home, tui: &mut support::Tui, text: &str) {
    let path = home.data().prompt_overrides().join(RULE);
    std::fs::write(path, text).expect("写得进去");
    tui.pump(Duration::from_millis(1500));
}

#[test]
fn a_changed_prefix_is_asked_and_send_anyway_or_cancel() {
    let home = Home::prepared(
        Script::new([
            Play::Says("一。"),
            Play::Says("二。"),
            Play::Says("不该说。"),
        ]),
        "",
        |root| {
            let dir = support::data_of(root).prompt_overrides().join("core");
            std::fs::create_dir_all(dir).expect("建得了目录");
        },
    );
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("第一句");
    tui.wait_for("一。");
    edit(&home, &mut tui, "Use absolute paths, always.\n");
    tui.say("第二句");
    tui.wait_for("下一轮的缓存会失效");
    assert!(
        tui.shows("local-paths-rule.txt"),
        "列出改了的文件：\n{}",
        tui.lines().join("\n")
    );
    tui.wait_for("[ 直接发送 ]");
    tui.key(b"\r");
    tui.wait_for("二。");
    edit(&home, &mut tui, "Use relative paths.\n");
    tui.say("第三句");
    tui.wait_for("下一轮的缓存会失效");
    tui.key(b"\x1b");
    let end = Instant::now() + support::WAIT;
    while tui.shows("下一轮的缓存会失效") {
        assert!(Instant::now() < end, "框没关：\n{}", tui.lines().join("\n"));
        tui.pump(Duration::from_millis(50));
    }
    tui.pump(Duration::from_millis(500));
    let screen = tui.lines().join("\n");
    assert!(!screen.contains("不该说"), "取消了不发：\n{screen}");
    assert!(screen.contains("第三句"), "字回到框里：\n{screen}");
    // 再发一次照样问（取消了的这一批没放行）；选「压缩上下文并发送」。
    tui.key(b"\r");
    tui.wait_for("下一轮的缓存会失效");
    tui.key(b"\x1b[C");
    tui.key(b"\r");
    // 才两轮，压不了：核心回压缩自己的拒绝，那句没发出去，照「没发出去」办（弹拒绝的话、字回到框里）。
    tui.wait_for("没有可压缩的内容");
    let screen = tui.lines().join("\n");
    assert!(!screen.contains("不该说"), "压不了就不发：\n{screen}");
    assert!(
        screen
            .lines()
            .any(|l| l.contains('❯') && l.contains("第三句")),
        "字回到框里：\n{screen}"
    );
}
