//! 伪终端里的端到端测试：一轮进行中来的回报不切断时间线（发版前 check 21，2026-11-09 项目主人从三个样板里挑了「放进时间线
//! 里」；蓝图 `tui.md`「时间线」第 21 条子条）：后台命令在一轮中间完成，收起以后只有一行，带「1 个后台命令完成」；点开能看到
//! 那一步。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 收起那一行认的字：中文、英文的收起行都认（收起行的语言照配置挑）。
const COUNTED: [&str; 2] = ["1 个后台命令完成", "1 background command finished"];

fn counted(line: &str) -> bool {
    COUNTED.iter().any(|w| line.contains(w))
}

/// 等到有一行收起行带着回报的数。
fn wait_counted(tui: &mut Tui) -> usize {
    let end = Instant::now() + support::WAIT;
    loop {
        if let Some(row) = tui.lines().iter().position(|l| counted(l)) {
            return row;
        }
        assert!(
            Instant::now() < end,
            "等不到带回报数的收起行：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(100));
    }
}

#[test]
fn a_report_inside_a_turn_is_a_step_of_the_timeline() {
    let bg = serde_json::json!({
        "command": "sleep 0.3; echo bg-done",
        "run_in_background": true,
        "description": "小睡一下"
    })
    .to_string();
    // 核心不让前台 `sleep` 等后台的（「Not run: no need to wait」，只认打头的词是 `sleep`）：照路径叫它占两秒。macOS 上没有
    // `timeout`。
    let wait = serde_json::json!({
        "command": "/bin/sleep 2; echo waited",
        "description": "等两秒"
    })
    .to_string();
    let after =
        serde_json::json!({"command": "echo after-report", "description": "收尾"}).to_string();
    let script = Script::new([
        Play::Calls(vec![("shell".into(), bg)]),
        Play::Calls(vec![("shell".into(), wait)]),
        Play::Calls(vec![("shell".into(), after)]),
        Play::Says("都做完了。"),
        // 核心要是把回报当下一轮的触发消息，另开的那一轮照这句答。
        Play::Says("收到回报。"),
    ]);
    let home = Home::with_tools(script, "[permission]\nlevel = \"full\"\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("做三件事");
    tui.wait_for("都做完了。");
    let row = wait_counted(&mut tui);
    let lines = tui.lines();
    let reply = lines
        .iter()
        .position(|l| l.contains("都做完了。"))
        .expect("回答在");
    // 回答以前只有一行收起行：回报没有单独一行，也没把时间线切成两段。
    let before: Vec<&String> = lines[..reply]
        .iter()
        .filter(|l| l.contains("后台命令完成") || counted(l))
        .collect();
    assert_eq!(before.len(), 1, "只有一行收起行：\n{}", lines.join("\n"));
    assert!(row < reply, "收起行在回答上面：\n{}", lines.join("\n"));
    // 点开那一段：回报是里面的一步，在两条命令中间。
    let click = format!("\x1b[<0;6;{}M\x1b[<0;6;{}m", row + 1, row + 1);
    tui.send(click.as_bytes());
    tui.wait_for("● 后台命令完成");
    let lines = tui.lines();
    let at = |text: &str| lines.iter().position(|l| l.contains(text));
    let (Some(report), Some(after)) = (at("● 后台命令完成"), at("echo after-report"))
    else {
        panic!("展开以后看得到回报和后面那一步：\n{}", lines.join("\n"));
    };
    assert!(
        report < after,
        "回报画在它发生的位置：\n{}",
        lines.join("\n")
    );
}
