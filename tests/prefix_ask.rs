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
    tui.wait_for("开放权限");
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

/// 收图的模型（同 `tests/refused.rs`）。
#[cfg(target_os = "linux")]
const SEEING: &str = "[providers.fake]\ndriver = \"openai-chat\"\nbase_url = \"http://127.0.0.1:9/v1\"\n\n[providers.fake.models.m]\ninputs = [\"text\", \"image\"]\n\n[models]\nchat = \"fake/m\"\n";

/// 数据根里记着 `said` 那一句的 `message.user`（会话日志一行一件，不管放在哪个目录）。
#[cfg(target_os = "linux")]
fn logged_message(root: &std::path::Path, said: &str) -> Option<serde_json::Value> {
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "jsonl") {
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                let found = text
                    .lines()
                    .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
                    .find(|e| {
                        e.to_string().contains("message.user") && e.to_string().contains(said)
                    });
                if found.is_some() {
                    return found;
                }
            }
        }
    }
    None
}

/// core1 11-06 核：被 `prefix_changing` 拒的那句核心什么都没记，重发要照原样带着附件（已经传好的 blob 引用，不再上传）。
#[test]
#[cfg(target_os = "linux")]
fn a_message_with_a_picture_keeps_it_when_sent_anyway() {
    let home = Home::prepared(
        Script::new([Play::Says("一。"), Play::Says("看到图了。")]),
        SEEING,
        |root| {
            let dir = support::data_of(root).prompt_overrides().join("core");
            std::fs::create_dir_all(dir).expect("建得了目录");
        },
    );
    let path = support::clipboard::picture(&home);
    let mut tui = support::clipboard::with_clipboard(&home, &path);
    tui.wait_for("Tab 切换权限级别");
    tui.say("第一句");
    tui.wait_for("一。");
    edit(&home, &mut tui, "Use absolute paths, always.\n");
    tui.key(b"\x16");
    tui.wait_for("[图片 1]");
    tui.say(" 看看这张");
    tui.wait_for("下一轮的缓存会失效");
    tui.key(b"\r");
    tui.wait_for("看到图了。");
    let message = logged_message(home.root(), "看看这张").expect("记着这一句");
    // 附件记成这一句的一块（带传好的 blob）。
    let blocks = message["body"]["blocks"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        blocks
            .iter()
            .any(|b| b["blob"].is_string() && b["name"] == "晚霞.png"),
        "重发以后图还在：{message}"
    );
}
