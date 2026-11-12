//! 伪终端里的端到端测试：输入历史重启以后还在、所有会话一起（蓝图 `tui.md`「输入历史列表」第 8 条，2026-10-07 项目主人定）。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

const UP: &[u8] = b"\x1b[A";

/// 输入框里那一行。
fn input(tui: &support::Tui) -> String {
    tui.lines()
        .into_iter()
        .find(|l| l.contains('❯'))
        .unwrap_or_default()
}

/// 按 ↑ 直到输入框里是 `text`。输入历史等核心交回软件包列表（状态目录在哪）才读进来，连上那一刻按的 ↑ 可能赶在它前面：
/// 没翻出来的收掉再按，最多等几秒（2026-10-11：本地十次红一次）。
fn up_until(tui: &mut support::Tui, text: &str) {
    let end = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        tui.key(UP);
        tui.pump(Duration::from_millis(300));
        if input(tui).contains(text) {
            return;
        }
        assert!(
            std::time::Instant::now() < end,
            "↑ 翻不到「{text}」：{}",
            tui.lines().join("\n")
        );
        tui.key(b"\x1b");
        tui.pump(Duration::from_millis(200));
    }
}

#[test]
fn what_was_sent_comes_back_with_up_after_a_restart_even_in_a_new_session() {
    let home = Home::new(Script::new([Play::Says("好。"), Play::Says("嗯。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("重启以前说的一句");
    tui.wait_for("好。");
    drop(tui);
    // 重启：新的界面、新的会话（ui.startup 照默认开新的）。
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    up_until(&mut tui, "重启以前说的一句");
    // 记在终端这个软件包的状态目录里（核心 9-1 上，`package.list` 的 `state`）。
    let saved =
        std::fs::read_to_string(home.data().package_state("tui").join("history.jsonl")).unwrap();
    assert_eq!(saved.lines().count(), 1, "记在包的状态目录里：{saved}");
}

/// 排期第 17 条（2026-11-03 照 Claude Code 改版）：`Ctrl+R` 每条左边时间、右边一行预览，下面露选中那条的全文，最下面是搜索
/// 那一行；直接打字就是模糊搜索（框线上不写「搜索：」），`Enter` 填进输入框。
#[test]
fn ctrl_r_shows_times_a_preview_and_a_search_row_and_typing_filters() {
    let home = Home::new(Script::new([Play::Says("好。"), Play::Says("嗯。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("帮我跑一下门禁");
    tui.wait_for("好。");
    tui.say("改一下侧边栏的顺序");
    tui.wait_for("嗯。");
    tui.pump(Duration::from_millis(300));
    tui.key(b"\x12");
    tui.wait_for("历史  2 条");
    tui.wait_for("筛选历史…");
    let lines = tui.lines();
    let row = |word: &str| {
        lines
            .iter()
            .position(|l| l.contains(word))
            .unwrap_or_else(|| panic!("没有「{word}」：\n{}", lines.join("\n")))
    };
    let list = row("刚才 ");
    assert!(
        lines[list].find("刚才").unwrap() < lines[list].find("门禁").unwrap(),
        "时间在左边：\n{}",
        lines.join("\n")
    );
    // 选中的是最新的那条：下面的预览又写了一遍它。
    let picked = lines
        .iter()
        .filter(|l| l.contains("改一下侧边栏的顺序") && l.contains('│'))
        .count();
    assert_eq!(picked, 2, "列表一行、预览一行：\n{}", lines.join("\n"));
    // 预览装在圆角框里（发版前 check 第 31 条），搜索那一行上面的横线接上左右的框线。
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.trim().starts_with('├') && l.trim().ends_with('┤'))
            .count(),
        1,
        "{}",
        lines.join("\n")
    );
    let inner = |corner: char| {
        lines
            .iter()
            .filter(|l| l.trim().starts_with('│') && l.contains(corner))
            .count()
    };
    assert_eq!((inner('╭'), inner('╰')), (1, 1), "{}", lines.join("\n"));
    assert!(
        row("筛选历史…") > row("改一下侧边栏的顺序"),
        "搜索那一行在最下面"
    );
    // 模糊搜索：「跑门」照先后对得上「帮我跑一下门禁」。
    tui.type_text("跑门");
    tui.wait_for("历史  1 条");
    assert!(
        !tui.shows("搜索："),
        "框线上不写搜索：\n{}",
        tui.lines().join("\n")
    );
    tui.key(b"\r");
    tui.pump(Duration::from_millis(300));
    assert!(
        input(&tui).contains("帮我跑一下门禁") && !tui.shows("历史  1 条"),
        "填进输入框、列表关了：\n{}",
        tui.lines().join("\n")
    );
}
