//! 伪终端里的端到端测试：配置页「软件包」接真的核心（蓝图 `tui.md`「配置页」第 43a 条）。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 从 `from` 那一项往下挪到 `to` 那一项、回车：主菜单一项两行（名字、说明）。
fn enter(tui: &mut support::Tui, from: &str, to: &str, lines_per_entry: usize) {
    let lines = tui.lines();
    let a = lines.iter().position(|l| l.contains(from)).expect("起点");
    let b = lines.iter().position(|l| l.contains(to)).expect("终点");
    for _ in 0..(b - a) / lines_per_entry {
        tui.key(b"\x1b[B");
    }
    tui.key(b"\r");
}

/// 联网搜索的 key、地址那一组归联网搜索（核心 F-14，2026-10-20 项目主人验收报：下面没有各家的 key 和地址）：
/// 列表里不单独成一个包，进了联网搜索那一页有「API Key 和地址」这一组。
#[test]
fn the_web_search_keys_live_on_the_web_search_page() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    tui.wait_for("运行日志");
    let menu = tui.lines().join("\n");
    assert!(menu.contains("联网搜索"), "{menu}");
    assert!(
        !menu.contains("、API Key 和地址"),
        "不单独成一个包：\n{menu}"
    );
    enter(&mut tui, "通用", "软件包", 2);
    tui.wait_for("联网搜索");
    let list = tui.lines();
    let first = list
        .iter()
        .position(|l| l.contains('┃'))
        .expect("选中的那一行");
    let net = list
        .iter()
        .position(|l| l.contains("联网搜索"))
        .expect("联网搜索");
    for _ in first..net {
        tui.key(b"\x1b[B");
    }
    tui.key(b"\r");
    tui.wait_for("搜索服务供应商");
    tui.wait_for("API Key 和地址");
}
