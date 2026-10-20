//! 伪终端里的端到端测试：刚发出去、她还没开始做事就两下 `Esc` 打断的，撤掉这一轮、那一句放回输入框（蓝图 `tui.md`
//! 「按键」`Esc`，2026-10-07 项目主人定）。真界面连一份照剧本回话的核心。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 输入框里那一行。
fn input(tui: &support::Tui) -> String {
    tui.lines()
        .into_iter()
        .find(|l| l.contains('❯'))
        .unwrap_or_default()
}

/// 运行状态行在：有一行最后是读秒（`容我想想.   3s`）。
fn running(tui: &support::Tui) -> bool {
    tui.lines().iter().any(|l| {
        let last = l.split_whitespace().last().unwrap_or_default();
        last.len() > 1
            && last.ends_with('s')
            && last[..last.len() - 1].chars().all(|c| c.is_ascii_digit())
    })
}

#[test]
fn an_interrupt_after_she_has_spoken_keeps_the_turn() {
    // 剧本的 `Holds` 先吐一个「…」再停住：她开口了，打断以后这一轮留着，那一句不放回输入框。
    let home = Home::new(Script::new([Play::Holds]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("说两句");
    tui.wait_for_line("…");
    tui.key(b"\x1b");
    tui.pump(Duration::from_millis(100));
    tui.key(b"\x1b");
    tui.wait_for("已中断");
    tui.pump(Duration::from_millis(500));
    assert!(
        tui.shows("┃ 说两句"),
        "这一轮留着：\n{}",
        tui.lines().join("\n")
    );
    assert!(!input(&tui).contains("说两句"), "不放回输入框");
}

#[test]
fn an_interrupt_before_she_starts_puts_the_message_back_in_the_box() {
    // `Stalls` 一个字都不出：她还没开始做事，打断以后撤掉这一轮，那一句放回输入框。
    let home = Home::new(Script::new([Play::Stalls, Play::Says("收到。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("等一下再问");
    // 等运行状态行的读秒出来（这一轮开始跑了）再按：只等固定的 800 毫秒，CI 忙的时候两下 Esc 都落在开轮以前，什么也
    // 打断不了（2026-10-14 Ubuntu CI 截到过）。
    let end = Instant::now() + support::WAIT;
    while !running(&tui) {
        assert!(
            Instant::now() < end,
            "这一轮没开始：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(100));
    }
    tui.key(b"\x1b");
    tui.pump(Duration::from_millis(100));
    tui.key(b"\x1b");
    let end = Instant::now() + support::WAIT;
    while !input(&tui).contains("等一下再问") {
        assert!(
            Instant::now() < end,
            "没放回输入框：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(100));
    }
    assert!(
        !tui.shows("┃ 等一下再问"),
        "这一轮撤掉了：\n{}",
        tui.lines().join("\n")
    );
    // 放回来的照常能再发。
    tui.key(b"\r");
    tui.wait_for("收到。");
}

#[test]
fn slash_stop_interrupts_and_keeps_the_queued_message_without_starting_a_turn() {
    // `/stop` 交给核心 `command.run`（2026-10-07 项目主人定）：打断这一轮，排着的话留在正文里、不接着发，回执弹提示。
    let home = Home::new(Script::new([Play::Holds, Play::Says("不该开这一轮。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("说两句");
    tui.wait_for_line("…");
    tui.say("排着的一句");
    tui.wait_for("↳ 排着的一句");
    tui.say("/stop");
    tui.wait_for("已全部停止");
    assert!(!tui.shows("已全部停止。"), "提示框里不要句号");
    tui.wait_for("已中断");
    tui.wait_for("┃ 排着的一句");
    tui.pump(Duration::from_millis(1500));
    let screen = tui.lines().join("\n");
    assert!(!screen.contains("不该开这一轮。"), "不接着开：\n{screen}");
    assert!(
        !screen.contains("↳ 排着的一句"),
        "不再画成排着的：\n{screen}"
    );
    let at = |word: &str| tui.lines().iter().position(|l| l.contains(word));
    assert!(
        at("已中断") < at("┃ 排着的一句"),
        "排着的话接在打断那一行后面：\n{screen}"
    );
}

#[test]
fn two_escapes_right_after_sending_stop_the_turn_once_it_starts() {
    // 2026-10-14 项目主人定：发出去以后、核心开轮以前按两下 `Esc` 也算打断，记下，轮一开就打断、那一句放回输入框
    // （原来这两下没交给打断那一处，这一轮照跑）。
    let home = Home::new(Script::new([Play::Stalls, Play::Says("收到。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("发错了");
    // 两下挨着按（隔 40 毫秒，读成两下、不读成 Alt+Esc），都落在开轮以前：先记下，轮一开再打断。
    tui.send(b"\x1b");
    tui.pump(Duration::from_millis(40));
    tui.send(b"\x1b");
    let end = Instant::now() + support::WAIT;
    while !input(&tui).contains("发错了") {
        assert!(
            Instant::now() < end,
            "没放回输入框：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(100));
    }
    assert!(
        !tui.shows("┃ 发错了"),
        "这一轮撤掉了：\n{}",
        tui.lines().join("\n")
    );
}

#[test]
fn a_line_sent_before_she_answers_a_word_joins_the_request_at_once() {
    // 核心 4-15（2026-10-14 项目主人定）：请求发出去、一个字都还没回时又发一句，核心撤掉那次、带上它重发，不再排着等
    // 下一步。剧本：第一次请求一个字都不回，重发的那一次回话。
    let home = Home::new(Script::new([Play::Stalls, Play::Says("两句都收到了。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("第一句");
    let end = Instant::now() + support::WAIT;
    while !running(&tui) {
        assert!(Instant::now() < end, "没开轮：\n{}", tui.lines().join("\n"));
        tui.pump(Duration::from_millis(100));
    }
    tui.say("第二句");
    tui.wait_for("两句都收到了。");
    tui.pump(Duration::from_millis(300));
    assert!(
        tui.shows("┃ 第二句") && !tui.shows("↳ 第二句"),
        "第二句进了正文、没排着：\n{}",
        tui.lines().join("\n")
    );
}

/// 没在回答时两下 `Esc` 不再清空输入框（2026-10-20 项目主人定），也不弹提示。
#[test]
fn two_escapes_while_idle_leave_the_typed_text_alone() {
    let home = Home::new(Script::new([]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.type_text("还没写完的一句");
    tui.wait_for("还没写完的一句");
    tui.key(b"\x1b");
    assert!(
        !tui.shows("再按一次 Esc"),
        "第一下不提示：{}",
        tui.lines().join("\n")
    );
    tui.key(b"\x1b");
    tui.pump(Duration::from_millis(300));
    assert!(
        input(&tui).contains("还没写完的一句"),
        "{}",
        tui.lines().join("\n")
    );
}
