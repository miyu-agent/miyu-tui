//! 找不到命令时交给她的兜底只接人在提示符上敲的那一行（2026-11-28 审计第二批 F1）：启动文件、提示符、命令替换里找不到的
//! 命令照 shell 原样报错。原来只看是不是交互式：启动文件末尾 `eval "$(没装的工具 init bash)"` 把她的回答当代码执行了，提示符
//! 里调了没装的命令的，每画一次提示符就交一轮。
#![cfg(unix)]

mod support;

use support::shell::{Hooked, SHELLS, installed};

/// 启动文件里、提示符里找不到的命令，各种 shell 的写法。
fn startup(shell: &str) -> &'static str {
    match shell {
        "bash" => {
            "PS1='$(nosuchgit-w1279 branch 2>/dev/null)P$ '\neval \"$(nosuchtool-w1279 init bash)\""
        }
        "zsh" => {
            "setopt prompt_subst\nPROMPT='$(nosuchgit-w1279 branch 2>/dev/null)P$ '\neval \"$(nosuchtool-w1279 init zsh)\""
        }
        _ => {
            "function fish_prompt; nosuchgit-w1279 branch 2>/dev/null; printf 'P$ '; end\nnosuchtool-w1279 init fish | source"
        }
    }
}

/// 命令行上的命令替换里调没装的命令。
fn substitution(shell: &str) -> &'static str {
    match shell {
        "fish" => "set out (nosuchsub-w1279 log); echo LEN=(string length -- \"$out\")",
        _ => "out=$(nosuchsub-w1279 log); echo LEN=${#out}",
    }
}

#[test]
fn only_the_line_typed_at_the_prompt_falls_back_to_her() {
    for shell in SHELLS.into_iter().filter(|s| installed(s)) {
        // 替身答的话里带一条命令：进了 `$(…)` 被执行，work/ 里就多出 PWNED。
        let mut sh = Hooked::start_with(shell, startup(shell), &[("answer", "touch PWNED\n")]);
        sh.tui.say("true");
        sh.wait_command_line("");
        sh.tui.say(substitution(shell));
        sh.wait_line(&format!("{shell} 命令替换跑完"), |line| line == "P$");
        assert!(
            sh.tui.shows("LEN=0"),
            "{shell}：命令替换里收进了她的回答：\n{}",
            sh.tui.lines().join("\n")
        );
        // 人敲的、首词带 `%` 的一句话：回车时判不出来照命令执行，找不到命令，照旧交给她。
        sh.tui.say("100%sure hello");
        let calls = sh.wait_calls(&format!("{shell} 交给她人敲的那一句"), |calls| {
            calls.iter().any(|c| c.ends_with("] 100%sure hello"))
        });
        assert_eq!(
            calls.len(),
            1,
            "{shell}：启动文件、提示符、命令替换里找不到的命令交给了她：{calls:?}"
        );
        assert!(
            !sh.work.join("PWNED").exists(),
            "{shell}：她的回答被当命令执行了"
        );
    }
}
