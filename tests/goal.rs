//! 伪终端里的端到端测试：长期目标（蓝图 `tui.md`「长期目标」，核心 X-8）接真的核心。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 她报完成：长期目标结束，不再开下一轮；下一段是她写的收尾（核心 X-8 上）。
fn completes() -> Play {
    let args = serde_json::json!({"action": "complete"}).to_string();
    Play::Calls(vec![("goal".into(), args)])
}

/// 首页打 `/goal <目标>` 先开会话（第 3 条）；核心开的续轮画成暗色一行，续轮消息的原文不露（第 4 条）。
#[test]
fn a_goal_set_on_the_home_page_opens_a_session_and_its_round_is_one_line() {
    let home = Home::with_tools(Script::new([completes(), Play::Says("测试都写完了。")]), "");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("/goal 把测试写完");
    tui.wait_for("已设定长期目标：把测试写完");
    tui.wait_for("● 长期目标 · 第 1 轮");
    tui.wait_for("测试都写完了。");
    let screen = tui.lines().join("\n");
    assert!(
        !screen.contains("goal_round") && !screen.contains("收到消息"),
        "不当别处来的话画：\n{screen}"
    );
}

/// 首页只打 `/goal`、`/goal pause` 不开会话，当场弹「无长期目标」（第 3 条）。
#[test]
fn looking_at_or_pausing_a_goal_on_the_home_page_opens_nothing() {
    let home = Home::new(Script::new([]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    for line in ["/goal", "/goal pause"] {
        tui.say(line);
        tui.wait_for("无长期目标");
        tui.pump(Duration::from_millis(300));
    }
    let sessions = home.root().join("home/alice/sessions");
    let opened = std::fs::read_dir(&sessions).map_or(0, Iterator::count);
    assert_eq!(opened, 0, "没开会话");
}

/// 会话开着、没有目标时停：只弹核心的原话，不写进正文（第 2 条）。
#[test]
fn pausing_without_a_goal_only_pops_the_cores_words() {
    let home = Home::with_tools(Script::new([Play::Says("你好。")]), "");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("你好");
    tui.wait_for("你好。");
    tui.say("/goal pause");
    tui.wait_for("没有长期目标");
    assert!(!tui.shows("核心拒绝"), "{}", tui.lines().join("\n"));
}

/// 长期目标没装：照核心的 `data.why` 说为什么（第 2 条）。
#[test]
fn a_goal_that_cannot_be_used_says_why() {
    let home = Home::with_tools(Script::new([Play::Says("你好。")]), "");
    // 卸掉出厂的长期目标（家目录记一笔，核心 F-5 上）；软件包核心起来时读，重启一次。
    let removed = home.root().join("home/alice/packages/goal.removed");
    std::fs::write(removed, "").expect("记得下");
    home.stop_core();
    home.restart_core();
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("你好");
    tui.wait_for("你好。");
    tui.say("/goal 把测试写完");
    tui.wait_for("长期目标未安装");
    assert!(!tui.shows("核心拒绝"), "{}", tui.lines().join("\n"));
}
