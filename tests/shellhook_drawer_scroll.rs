//! shell 里确认一条长命令时 `Ctrl+J` `Ctrl+K` 滚命令那一块（2026-11-04 项目主人报：shellhook 里滚不动；全屏照 `app/vim.rs` 早就
//! 归抽屉滚，shell 里那一条路先把它们当成了上下、又交给了外面）。
#![cfg(unix)]

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn ctrl_j_and_k_scroll_a_long_command_in_the_shell_drawer() {
    let command: Vec<String> = (1..=40).map(|n| format!("echo line-{n:02}")).collect();
    let args =
        serde_json::json!({"command": command.join("\n"), "description": "数四十行"}).to_string();
    let home = Home::with_tools(
        Script::new([
            Play::Calls(vec![("shell".into(), args)]),
            Play::Says("好。"),
        ]),
        "",
    );
    let input = home.root().join("said");
    std::fs::write(&input, "数一下").expect("写得进去");
    let input = input.display().to_string();
    let mut tui = home.tui_args_with("zh_CN.UTF-8", &["shellhook", "say", "--input", &input], &[]);
    tui.wait_for("允许本次");
    // 只看抽屉框里那几行：上面时间线的预览里也有命令。
    let drawer = |t: &support::Tui, word: &str| {
        let lines = t.lines();
        let top = lines
            .iter()
            .position(|l| l.starts_with('╭'))
            .unwrap_or(lines.len());
        lines[top..].iter().any(|l| l.contains(word))
    };
    assert!(drawer(&tui, "echo line-01"), "{}", tui.lines().join("\n"));
    let wait = |tui: &mut support::Tui, ok: &dyn Fn(&support::Tui) -> bool, what: &str| {
        let end = Instant::now() + Duration::from_secs(5);
        while !ok(tui) {
            assert!(Instant::now() < end, "{what}：\n{}", tui.lines().join("\n"));
            tui.pump(Duration::from_millis(50));
        }
    };
    // Ctrl+J：往下滚一行，第一行滚出去；选中的还是原来那一项（不当 ↓）。
    tui.key(b"\n");
    wait(&mut tui, &|t| !drawer(t, "echo line-01"), "Ctrl+J 往下滚");
    // Ctrl+K：滚回来。
    tui.key(b"\x0b");
    wait(&mut tui, &|t| drawer(t, "echo line-01"), "Ctrl+K 往上滚");
    assert!(tui.shows("允许本次"), "抽屉还开着");
}
