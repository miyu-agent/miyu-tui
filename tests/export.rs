//! 伪终端里的端到端测试：`/export`（排期第 36 条，核心 SE-1；蓝图 `tui.md`「导出 `/export`」）：先选给谁用，三种 Markdown 估出
//! 大小和 token；自己看的存成文件写进终端所在的目录；存档直接存成 ZIP。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 等终端所在的目录里出现 `name`。
fn wait_file(tui: &mut support::Tui, path: &std::path::Path) {
    let end = Instant::now() + support::WAIT;
    while !path.exists() {
        assert!(
            Instant::now() < end,
            "没写出 {}：\n{}",
            path.display(),
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(50));
    }
}

#[test]
fn export_estimates_each_use_and_saves_a_markdown_and_a_bundle() {
    let home = Home::new(Script::new([Play::Says("记下了。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("帮我记一下导出测试");
    tui.wait_for("记下了。");
    tui.say("/export");
    tui.wait_for("完整存档");
    // 四种用途的名字（发版前 check 22，2026-11-09 项目主人改的）。
    for name in [
        "用于给别的 agent 继续",
        "用于网页端聊天对话",
        "仅导出对话为 Markdown 文件",
    ] {
        tui.wait_for(name);
    }
    // 三种 Markdown 都估出来了（「· 约 N tokens」）。
    let end = Instant::now() + support::WAIT;
    while tui.lines().iter().filter(|l| l.contains("tokens")).count() < 3 {
        assert!(Instant::now() < end, "没估完：\n{}", tui.lines().join("\n"));
        tui.pump(Duration::from_millis(50));
    }
    // 自己看（第四行）→ 存成文件（第二行）。
    for _ in 0..3 {
        tui.key(b"\x1b[B");
    }
    tui.key(b"\r");
    tui.wait_for("存成文件");
    tui.key(b"\x1b[B");
    tui.key(b"\r");
    let id = std::fs::read_dir(home.data().sessions_dir())
        .expect("有会话目录")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .next()
        .expect("开了一个会话");
    let short: String = id
        .chars()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let md = home.work.join(format!("miyu-{short}-read.md"));
    wait_file(&mut tui, &md);
    tui.wait_for("已导出到");
    let text = std::fs::read_to_string(&md).expect("读得到");
    assert!(
        text.contains("帮我记一下导出测试") && text.contains("记下了。"),
        "{text}"
    );
    // 存档：第一行直接存成 ZIP。
    tui.say("/export");
    tui.wait_for("完整存档");
    tui.key(b"\r");
    let zip = home.work.join(format!("miyu-{short}.zip"));
    wait_file(&mut tui, &zip);
    assert!(
        std::fs::read(&zip).expect("读得到").starts_with(b"PK"),
        "是 ZIP"
    );
}
