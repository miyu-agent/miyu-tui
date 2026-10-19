//! 伪终端里的端到端测试：配置页「软件包」接真的核心（蓝图 `tui.md`「配置页」第 43a 条）。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 往下挪到选中的那一行（行首 `┃`）是 `to`、回车：每按一下看一眼，慢的机器上按键早到也不会多挪、少挪。
fn enter(tui: &mut support::Tui, to: &str) {
    for _ in 0..30 {
        if tui
            .lines()
            .iter()
            .any(|l| l.contains('┃') && l.contains(to))
        {
            tui.key(b"\r");
            return;
        }
        tui.key(b"\x1b[B");
    }
    panic!("挪不到「{to}」：\n{}", tui.lines().join("\n"));
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
    enter(&mut tui, "软件包");
    tui.wait_for("联网搜索");
    enter(&mut tui, "联网搜索");
    tui.wait_for("搜索服务供应商");
    tui.wait_for("API Key 和地址");
}
