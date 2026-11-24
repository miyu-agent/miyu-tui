//! shell 里她答话时粘贴（2026-11-25 查出：shell 集成从来没开括号粘贴，多行粘贴在任何终端上都拆成几次插话）：起来时开括号
//! 粘贴、收工时关；粘进来的多行字整段归输入那一行，交还给 shell 时还是那几行。`Ctrl+H` 照退格删字（`reader.rs`）。
#![cfg(unix)]

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 开、关括号粘贴的两句。
const PASTE_ON: &[u8] = b"\x1b[?2004h";
const PASTE_OFF: &[u8] = b"\x1b[?2004l";

fn has(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn a_pasted_block_stays_whole_and_paste_mode_is_switched_back_off() {
    // 她开了个头就停住，等叫停：这期间粘的、敲的都归输入那一行。
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
    tui.record();
    tui.pump(Duration::from_millis(1500));
    assert!(has(&tui.recorded(), PASTE_ON), "起来时开了括号粘贴");
    tui.record();
    tui.key(b"\x1b[200~line one\nline two\x1b[201~");
    // 再敲一个字、用 Ctrl+H 删掉：照退格。
    tui.type_text("x");
    tui.key(b"\x08");
    // Ctrl+C 打断：这一轮停下，敲的字交还给 shell。
    tui.key(b"\x03");
    let end = Instant::now() + Duration::from_secs(15);
    while !leftover.exists() {
        assert!(Instant::now() < end, "没交还：\n{}", tui.lines().join("\n"));
        tui.pump(Duration::from_millis(100));
    }
    tui.pump(Duration::from_millis(300));
    assert_eq!(
        std::fs::read_to_string(&leftover).expect("读得到"),
        "line one\nline two",
        "两行整段交还，没被拆成插话；Ctrl+H 删掉了 x"
    );
    assert!(has(&tui.recorded(), PASTE_OFF), "收工时关了括号粘贴");
}
