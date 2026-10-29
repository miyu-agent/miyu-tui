//! 伪终端里的端到端测试：长期目标（蓝图 `tui.md`「长期目标」，核心 X-8）接真的核心。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 她报完成：长期目标结束，不再开下一轮；下一段是她写的收尾（核心 X-8 上）。
fn completes() -> Play {
    let args = serde_json::json!({"action": "complete"}).to_string();
    Play::Calls(vec![("goal".into(), args)])
}

/// 框上面那一行目标进度的开头（2026-10-14 项目主人定：打头一个 `◎`）：正文里「● 长期目标 · 第 1 轮」那一行不算。
const TAG: &str = "◎ 目标 · ";

/// 等到屏幕上没有这串字，最多 [`support::WAIT`]。
fn wait_gone(tui: &mut support::Tui, text: &str) {
    let end = Instant::now() + support::WAIT;
    while tui.shows(text) {
        assert!(
            Instant::now() < end,
            "「{text}」一直在：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(50));
    }
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
    // 第 5 条：做完了，框上面不再写目标。
    wait_gone(&mut tui, TAG);
}

/// 第 5 条：跑着的，框上面紧挨着的那一行右对齐写「◎ 目标 · 进行中 · 第 1/5 轮」（核心推来的 `goal.changed`）；上边框右边
/// 是标题的位置，不写目标（2026-10-14 项目主人定）。
#[test]
fn a_running_goal_shows_right_above_the_input_box() {
    let home = Home::with_tools(Script::new([Play::Holds]), "");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("/goal --rounds 5 把测试写完");
    tui.wait_for("◎ 目标 · 进行中 · 第 1/5 轮");
    let lines = tui.lines();
    let at = lines
        .iter()
        .position(|l| l.contains("◎ 目标 · 进行中 · 第 1/5 轮"))
        .expect("有那一行");
    let border = &lines[at + 1];
    assert!(
        border.contains('╭') && !border.contains("目标"),
        "紧挨着下面是上边框，边框上不写目标：\n{}",
        lines.join("\n")
    );
    // 右对齐：右端落在框里放字那一块的右边，也就是右边框往里让出右内边距（框下面那一行的用量也对齐到这里）。
    let end = |l: &str| unicode_width::UnicodeWidthStr::width(l.trim_end());
    let gap = end(border) - end(&lines[at]);
    assert!(
        (1..=4).contains(&gap),
        "右端离右边框 {gap} 列：\n{}",
        lines.join("\n")
    );
}

/// 第 5 条：续轮里她没调工具、等人开口的写「等待回复」；切到新会话不写，切回来照订阅回应里的 `goal` 写回来。
#[test]
fn a_goal_waiting_for_a_reply_comes_back_with_its_session() {
    let home = Home::with_tools(Script::new([Play::Says("要你定一下用哪个库。")]), "");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("/goal 把测试写完");
    tui.wait_for("要你定一下用哪个库。");
    tui.wait_for("目标 · 等待回复 · 第 1 轮");
    tui.say("/new");
    wait_gone(&mut tui, TAG);
    tui.say("/sessions");
    tui.wait_for("1 个");
    tui.key(b"\r");
    tui.wait_for("目标 · 等待回复 · 第 1 轮");
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
    let sessions = home.root().join("sessions");
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

/// 长期目标没装：核心 BS-1下下 起没有这条命令（项目主人 10-27 定）；列表里不列，打了核心说不认得，同别的不存在的命令不放回
/// 输入框（2026-10-23 项目主人）。
#[test]
fn a_goal_without_its_package_is_no_command() {
    let home = Home::with_tools(Script::new([Play::Says("你好。")]), "");
    // 卸掉出厂的长期目标（系统那一层记一笔，核心 F-5 上；SU-5 上起在 `packages/`）；软件包核心起来时读，重启一次。
    let removed = home.root().join("packages/goal.removed");
    std::fs::write(removed, "").expect("记得下");
    home.stop_core();
    home.restart_core();
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("你好");
    tui.wait_for("你好。");
    tui.say("/goal 把测试写完");
    tui.wait_for("命令不存在");
    assert!(!tui.shows("核心拒绝"), "{}", tui.lines().join("\n"));
    assert!(
        !tui.shows("❯ /goal 把测试写完"),
        "不放回输入框：\n{}",
        tui.lines().join("\n")
    );
}

/// 改目标的用法（2026-10-17 项目主人：打 /goal 按 `↑` 调出之前那句改了回车，核心 X-8 再补起再发就是改）：命令列表开着时
/// `↑` 也照打的字开头翻输入历史，别的话跳过；`Ctrl+K` 照旧在列表里挪。
#[test]
fn typing_goal_then_up_brings_back_the_last_goal_with_its_words() {
    let home = Home::with_tools(Script::new([Play::Holds]), "");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("/goal 把测试写完");
    tui.wait_for("已设定长期目标");
    // 命令列表开着时 ↑ 只在列表里挪（2026-10-20 项目主人）；打了空格、列表收起以后再翻（同一天项目主人选的）。
    tui.type_text("/goal ");
    tui.key(b"\x1b[A");
    tui.wait_for("❯ /goal 把测试写完");
    assert!(
        !tui.shows("命令 1 条"),
        "翻出来的不弹命令列表：\n{}",
        tui.lines().join("\n")
    );
    tui.key(b"\x1b[B");
    let end = std::time::Instant::now() + support::WAIT;
    while tui.shows("把测试写完") && tui.shows("❯ /goal 把") {
        assert!(
            std::time::Instant::now() < end,
            "↓ 回到打的字：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(std::time::Duration::from_millis(50));
    }
}
