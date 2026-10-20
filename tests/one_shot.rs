//! 伪终端里的端到端测试：只办一条命令（蓝图 `tui.md`「只办一条命令」，shellhook 借终端界面的框）。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 开几个会话（各说一句），交回按编号排的会话编号。
fn sessions(home: &Home, n: usize) -> Vec<String> {
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    for i in 1..=n {
        if i > 1 {
            tui.say("/new");
            tui.pump(Duration::from_millis(200));
        }
        tui.say(&format!("第 {i} 号的话"));
        tui.wait_for(&format!("第 {i} 号的回答。"));
        tui.pump(Duration::from_millis(200));
    }
    drop(tui);
    let mut ids: Vec<String> = std::fs::read_dir(home.root().join("home/alice/sessions"))
        .expect("会话目录")
        .map(|e| {
            e.expect("读得出目录项")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    ids.sort();
    assert_eq!(ids.len(), n);
    ids
}

fn home(n: usize) -> Home {
    let plays: Vec<Play> = (1..=n)
        .map(|i| Play::Says(Box::leak(format!("第 {i} 号的回答。").into_boxed_str())))
        .collect();
    Home::new(Script::new(plays))
}

fn one_shot(home: &Home, id: &str, line: &str) -> (Tui, std::path::PathBuf) {
    let report = home.root().join("report");
    let report_arg = report.to_string_lossy().into_owned();
    let tui = home.tui_args(
        "zh_CN.UTF-8",
        &["--resume", id, "--command", line, "--report", &report_arg],
    );
    (tui, report)
}

#[test]
fn the_box_closing_ends_the_program_and_nothing_is_reported_when_the_session_stays() {
    let home = home(1);
    let ids = sessions(&home, 1);
    let (mut tui, report) = one_shot(&home, &ids[0], "/status");
    tui.wait_for("会话 ID");
    tui.key(b"\x1b");
    assert_eq!(tui.exited(), Some(0), "框关了就退出");
    assert!(!report.exists(), "会话没换，不写");
}

#[test]
fn picking_another_session_reports_it() {
    let home = home(2);
    let ids = sessions(&home, 2);
    let (mut tui, report) = one_shot(&home, &ids[0], "/sessions");
    tui.wait_for("2 个");
    // 正在用的排第一行，下一行是另一个。
    tui.key(b"\x1b[B");
    tui.key(b"\r");
    assert_eq!(tui.exited(), Some(0));
    let reported = std::fs::read_to_string(&report).expect("会话换了，写编号");
    assert_eq!(reported.trim(), ids[1]);
}
