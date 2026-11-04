//! 伪终端里的端到端测试：导进来的只读存档（排期第 38 条，核心设计 58 第三节、SE-1 下；蓝图 `tui.md`「会话列表」第 3 条子条、
//! 「输入框」第 9 条子条）：打开存档时输入框空着写一行「这是存档」；还是要说的，核心拒 `archived`，字退回框里、弹提示。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn an_archive_says_so_in_the_input_box_and_refuses_to_continue() {
    let home = Home::new(Script::new([Play::Says("你好呀。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("在吗");
    tui.wait_for("▣  ");
    // 这个会话当成导进来的存档（核心认会话目录里的 `imported.json`）：重启核心，界面重连、重读会话列表。
    let sessions = home.data().sessions_dir();
    let dir = std::fs::read_dir(&sessions)
        .expect("有会话目录")
        .filter_map(Result::ok)
        .find(|e| e.path().is_dir())
        .expect("开了一个会话")
        .path();
    std::fs::write(
        dir.join("imported.json"),
        r#"{"archive":true,"miyu":"0.0.0","exported":"2026-11-04T00:00:00Z"}"#,
    )
    .expect("写得进去");
    home.stop_core();
    home.restart_core();
    tui.wait_for("这是存档");
    assert!(tui.shows("你好呀。"), "以前的对话照旧画着");
    // 还是要说：核心拒 `archived`，弹提示，字退回框里。
    tui.type_text("再说一句");
    tui.key(b"\r");
    tui.wait_for("这是存档，不能接着说");
    assert!(tui.shows("再说一句"), "{}", tui.lines().join("\n"));
}
