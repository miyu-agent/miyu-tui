//! 握手报的这一头（核心 VN-1，蓝图 `shellhook.md`「连核心」）：全屏的终端界面报 `tui`，终端无缝集成报 `shellhook`。核心把它
//! 记进人说的那一句的 `by.head`，AI 照它知道话是从哪来的。原来两边都报 `tui`，在 shell 里问 AI 在哪说话，它答终端界面。
#![cfg(unix)]

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use serde_json::{Value, json};
use support::Home;

/// 会话日志里记着 `said` 那一句的 `by`：落盘可能比回答晚一点，等一会儿。
fn by_of(home: &Home, said: &str) -> Value {
    let end = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(message) = support::clipboard::logged_message(home.root(), said) {
            return message["by"].clone();
        }
        assert!(Instant::now() < end, "没记下「{said}」");
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn what_the_shellhook_says_is_marked_as_said_in_the_shellhook() {
    let home = Home::new(Script::new([Play::Says("在 shell 里。")]));
    let input = home.root().join("said");
    std::fs::write(&input, "你在哪和我说话").expect("写得进去");
    let input = input.display().to_string();
    let mut tui = home.tui_args_with("zh_CN.UTF-8", &["shellhook", "say", "--input", &input], &[]);
    tui.wait_for("在 shell 里。");
    assert_eq!(
        by_of(&home, "你在哪和我说话"),
        json!({"kind": "person", "head": "shellhook"})
    );
}

#[test]
fn what_the_full_screen_says_is_marked_as_said_in_the_tui() {
    let home = Home::new(Script::new([Play::Says("在终端界面里。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    tui.say("你在哪和我说话");
    tui.wait_for("在终端界面里。");
    assert_eq!(
        by_of(&home, "你在哪和我说话"),
        json!({"kind": "person", "head": "tui"})
    );
}
