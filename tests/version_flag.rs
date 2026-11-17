//! `--version`、`-V`（2026-11-17 项目主人本机验收第 11 条：`miyu tui --version` 印了一大段 `Error: Custom { … }`）：程序自己
//! 印 `miyu tui <版本>`、`miyu shellhook <版本>`，退出码 0，同核心 VS-1 替包的命令答的那一行；不认的参数印用法到标准错误、
//! 退出码 2，不再印成 `Error: Custom`。

use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_miyu-tui"))
        .args(args)
        .env(
            "MIYU_HOME",
            std::env::temp_dir().join("miyu-tui-version-flag-unused"),
        )
        .output()
        .expect("跑得起来")
}

#[test]
fn the_version_flags_print_one_line_and_exit_zero() {
    let version = env!("CARGO_PKG_VERSION");
    for (args, want) in [
        (&["--version"][..], format!("miyu tui {version}\n")),
        (&["-V"], format!("miyu tui {version}\n")),
        (
            &["shellhook", "--version"],
            format!("miyu shellhook {version}\n"),
        ),
        (&["shellhook", "-V"], format!("miyu shellhook {version}\n")),
    ] {
        let out = run(args);
        assert_eq!(out.status.code(), Some(0), "{args:?}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), want, "{args:?}");
    }
}

#[test]
fn an_unknown_argument_prints_the_usage_not_an_error_value() {
    let out = run(&["--bogus"]);
    assert_eq!(out.status.code(), Some(2));
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(
        said.contains("Usage: miyu-tui") && !said.contains("Custom"),
        "{said}"
    );
}
