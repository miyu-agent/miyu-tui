//! shell 里 `/edit` 放回命令行的那一句（2026-11-28 审计第二批 F2）：回车一律交给她、替换上一轮重来，不先判是不是命令。
//! 原来先判：在全屏里说的「touch 报错日志 之后编译报错了怎么办」放回来，首词像命令，回车被 shell 当命令执行了。
#![cfg(unix)]

mod support;

use support::shell::{Hooked, SHELLS, installed};

#[test]
fn the_sentence_put_back_by_edit_is_said_again_even_if_it_starts_like_a_command() {
    for shell in SHELLS.into_iter().filter(|s| installed(s)) {
        let mut sh = Hooked::start(shell, "");
        sh.tui.say("/edit");
        // 替身照真程序把「touch edited」放回命令行、旁边放 `.redo`。
        sh.wait_command_line("touch edited");
        sh.tui.send(b"\r");
        let calls = sh.wait_calls(&format!("{shell} 交给她第二句"), |calls| {
            calls.len() >= 2
        });
        assert!(
            calls[1].contains("--redo") && calls[1].ends_with("] touch edited"),
            "{shell}：放回来的那一句带着 --redo 交给她：{calls:?}"
        );
        sh.wait_command_line("");
        assert!(
            !sh.work.join("edited").exists(),
            "{shell}：放回来的那一句被当命令执行了"
        );
    }
}
