//! 伪终端里的端到端测试：一句话超过协议一行（1 MiB，施工 YC-11，压测报告 F5）。贴两 MB 的日志回车：整段分块传给核心存成
//! 文字附件（`blob.open`、`blob.write`、`blob.close`），话发出去、框清空、正文里照附件写名字，核心把它交给模型。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn two_megabytes_pasted_go_out_as_a_text_attachment() {
    let script = Script::new([Play::Says("收到日志。")]);
    let home = Home::new(script.clone());
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    let log = "2026-11-10 10:00:00 ERROR 连不上 upstream\n".repeat(50_000);
    tui.send(format!("\x1b[200~{log}\x1b[201~").as_bytes());
    tui.wait_for("[已粘贴 50000 行]");
    tui.pump(Duration::from_millis(300));
    tui.send(b"\r");
    tui.wait_for("收到日志。");
    let screen = tui.lines().join("\n");
    assert!(
        screen.contains("[pasted-"),
        "正文里照附件写名字：\n{screen}"
    );
    assert!(!tui.input().contains("已粘贴"), "框清空：{}", tui.input());
    let sent = format!("{:?}", script.requests().last());
    assert!(
        sent.contains("pasted-"),
        "模型那头见到这个附件：{}",
        &sent[..sent.len().min(2000)]
    );
}

/// shell 里也一样（`shellhook say`）：两 MB 的一句转成文字附件发出去，下面印一句转成了哪个附件。
#[test]
fn the_shell_sends_two_megabytes_as_a_text_attachment_too() {
    let script = Script::new([Play::Says("收到日志。")]);
    let home = Home::new(script.clone());
    let input = home.root().join("said");
    let log = "2026-11-10 10:00:00 ERROR 连不上 upstream\n".repeat(50_000);
    std::fs::write(&input, &log).expect("写得进去");
    let input = input.display().to_string();
    let mut tui = home.tui_args_with("zh_CN.UTF-8", &["shellhook", "say", "--input", &input], &[]);
    tui.wait_for("收到日志。");
    let screen = tui.lines().join("\n");
    assert!(
        screen.contains("转成附件 pasted-"),
        "印了转成了哪个附件：\n{screen}"
    );
    let sent = format!("{:?}", script.requests().last());
    assert!(
        sent.contains("pasted-"),
        "模型那头见到这个附件：{}",
        &sent[..sent.len().min(2000)]
    );
}
