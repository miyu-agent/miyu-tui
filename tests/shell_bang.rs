//! 伪终端里的端到端测试：人自己跑命令 `!`（排期第 21 条，核心 SH-1；蓝图 `tui.md`「人自己跑命令 `!`」）：空框里打 `!` 进跑命令，
//! 回车照会话的工作目录跑，正文画「! 命令」、输出、退出码，她接着回一轮；输入历史里记成 `!` 打头的一条。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn a_bang_command_runs_in_the_session_and_she_answers() {
    let home = Home::with_tools(
        Script::new([Play::Says("看到输出了。"), Play::Says("又看到了。")]),
        "[permission]\nlevel = \"full\"\n",
    );
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.type_text("!");
    tui.wait_for("输入要跑的命令");
    tui.type_text("echo hi-from-bang");
    tui.key(b"\r");
    tui.wait_for("退出码 0");
    tui.wait_for("看到输出了。");
    let screen = tui.lines().join("\n");
    assert!(screen.contains("! echo hi-from-bang"), "{screen}");
    assert!(
        screen
            .lines()
            .any(|l| l.trim().ends_with("hi-from-bang") && !l.contains("echo")),
        "输出画在命令下面：\n{screen}"
    );
    // 发出去了回到平常的输入；`↑` 翻出来的照样进跑命令：再回车又跑一次。
    assert!(!tui.shows("输入要跑的命令"), "{screen}");
    tui.key(b"\x1b[A");
    tui.wait_for("echo hi-from-bang");
    tui.key(b"\r");
    tui.wait_for("又看到了。");
    let screen = tui.lines().join("\n");
    assert_eq!(screen.matches("退出码 0").count(), 2, "{screen}");
}

/// 核心 XG-1 下（2026-11-06 core1）：会话里没有内置 shell 的，`!` 发出去回 `shell_unavailable`：弹提示「这个会话不能跑命令」，
/// 不写进正文，打的命令放回框里。
#[test]
fn a_bang_command_without_a_shell_says_so_and_keeps_the_command() {
    let home = Home::new(Script::new([Play::Says("不该轮到我。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.type_text("!");
    tui.wait_for("输入要跑的命令");
    tui.type_text("echo no-shell-here");
    tui.key(b"\r");
    tui.wait_for("这个会话不能跑命令");
    let screen = tui.lines().join("\n");
    assert!(!screen.contains("退出码"), "没跑：\n{screen}");
    assert!(!screen.contains("不该轮到我"), "她没接着说：\n{screen}");
    assert!(
        screen.contains("echo no-shell-here"),
        "命令放回框里：\n{screen}"
    );
}
