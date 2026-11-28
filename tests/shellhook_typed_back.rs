//! 她答话、后台回报正在印的时候你敲的字，印完放回命令行（2026-11-28 审计第二批 F3）。原来回车交给她的那条路放得回，
//! 找不到命令交给她、后台回报这几条路读走了就丢了：bash、fish 找不到命令那条路没带 `--leftover`；zsh 带了，可处理函数跑在
//! 为这条命令分出来的子进程里，`print -z` 推不回父 shell；bash、zsh 的后台回报没带 `--leftover`。
#![cfg(unix)]

mod support;

use support::shell::{Hooked, SHELLS, installed};

/// 替身照「你敲了这几个字」交还的。
const TYPED: &str = "abc";

#[test]
fn keys_typed_while_she_answers_a_command_not_found_come_back() {
    for shell in SHELLS.into_iter().filter(|s| installed(s)) {
        let mut sh = Hooked::start_with(shell, "", &[("typed", TYPED)]);
        // 首词带 `%`：回车时判不出来照命令执行，找不到命令，交给她。
        sh.tui.say("100%sure hello");
        sh.wait_calls(&format!("{shell} 交给她"), |calls| calls.len() == 1);
        sh.wait_command_line(TYPED);
    }
}

#[test]
fn keys_typed_while_a_report_prints_at_the_prompt_come_back() {
    for shell in SHELLS.into_iter().filter(|s| installed(s)) {
        let mut sh = Hooked::start_with(shell, "", &[("typed", TYPED)]);
        sh.set("mail", "");
        sh.signal_report();
        // bash 停在提示符上不办信号：等你回车、命令跑以前才印（蓝图「后台任务和回报」第 5 条）。
        if shell == "bash" {
            sh.tui.say("true");
        }
        sh.wait_calls(&format!("{shell} 印回报"), |calls| {
            calls.iter().any(|c| c.starts_with("report "))
        });
        sh.wait_command_line(TYPED);
    }
}

#[test]
fn keys_typed_while_a_report_prints_after_her_answer_come_back() {
    for shell in SHELLS.into_iter().filter(|s| installed(s)) {
        // 她答话时来了后台回报（替身在答话里放一件、发信号）：答完接着印，印的时候你又敲了字。
        let mut sh = Hooked::start_with(shell, "", &[("wake", ""), ("typed", TYPED)]);
        sh.tui.say("hello there");
        sh.wait_calls(&format!("{shell} 印回报"), |calls| {
            calls.iter().any(|c| c.starts_with("report "))
        });
        // 答话那一下敲的、印回报那一下敲的，都回到命令行上。
        sh.wait_command_line(&format!("{TYPED}{TYPED}"));
    }
}
