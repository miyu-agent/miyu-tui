//! shell 里她答话时你敲的字照全屏输入框编辑（排期第 46 条，2026-11-04 项目主人：「光标没法左右移动也没法换行」「行为应该和全屏完全
//! 相同才对的」）：敲 `ac`、按 ←、再敲 `b`，交还给 shell 的是 `abc`（原来只能往末尾加字，交还的是 `acb`）。
#![cfg(unix)]

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn keys_typed_while_she_answers_move_the_cursor_like_the_full_screen_box() {
    // 她开了个头就停住，等叫停：这期间敲的字都归输入那一行。
    let home = Home::new(Script::new([Play::Holds]));
    let input = home.root().join("said");
    std::fs::write(&input, "等着").expect("写得进去");
    let leftover = home.root().join("leftover");
    let (input, out) = (input.display().to_string(), leftover.display().to_string());
    let mut tui = home.tui_args_with(
        "zh_CN.UTF-8",
        &["shellhook", "say", "--input", &input, "--leftover", &out],
        &[],
    );
    tui.pump(Duration::from_millis(1500));
    tui.type_text("ac");
    tui.key(b"\x1b[D");
    tui.type_text("b");
    tui.wait_for("› abc");
    // Ctrl+C 打断：这一轮停下，敲的字交还给 shell。
    tui.key(b"\x03");
    let end = Instant::now() + Duration::from_secs(15);
    while !leftover.exists() {
        assert!(Instant::now() < end, "没交还：\n{}", tui.lines().join("\n"));
        tui.pump(Duration::from_millis(100));
    }
    tui.pump(Duration::from_millis(200));
    assert_eq!(std::fs::read_to_string(&leftover).expect("读得到"), "abc");
}
