//! 推演人生（蓝图 `tui.md`「推演人生」，核心 LF-8）：伪终端里的真界面连一份照剧本回话的核心。`/life new` 打了就开、跳到人生
//! 会话，会话列表认得出它；人格的窗里的开关先问一句再开，开着的回车暂停。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 人生会话一开就醒一次（核心 `life.start`）：给她几句话回。
fn script(first: &'static str) -> Script {
    Script::new([
        Play::Says(first),
        Play::Says("在看书。"),
        Play::Says("在看书。"),
    ])
}

/// 等 `check` 成立，最多 [`support::WAIT`]；等不到的把屏幕打出来。
fn until(tui: &mut Tui, what: &str, check: impl Fn(&Tui) -> bool) {
    let end = Instant::now() + support::WAIT;
    while !check(tui) {
        assert!(
            Instant::now() < end,
            "等不到{what}，屏幕是：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(100));
    }
}

#[test]
fn life_new_jumps_to_the_life_session_and_the_list_marks_it() {
    let home = Home::with_settings(script("好。"), "[tui]\nicons = \"plain\"\n");
    // 人生会话照会话的人格开：这个会话带上出厂的人格「软件工程师」
    let mut tui = home.tui_wide_with(
        "zh_CN.UTF-8",
        160,
        &[("MIYU_TUI_PERSONA", "software-engineer")],
    );
    tui.wait_for("开放权限");
    tui.say("在吗");
    tui.wait_for_done();
    assert!(tui.shows("┃ 在吗"), "{}", tui.lines().join("\n"));
    // 打了就开，不问（项目主人：「只网页确认」）；回应带着人生会话的编号，跳过去。
    tui.say("/life new");
    until(&mut tui, "跳到人生会话", |t| !t.shows("┃ 在吗"));
    // 会话列表里，人生会话的标题前面一个记号（图标 plain 那一套是 `♡`）。
    tui.say("/sessions");
    tui.wait_for("♡ ");
}

#[test]
fn the_persona_window_asks_before_turning_life_on_and_pauses_it_without_asking() {
    let home = Home::new(script("在看书。"));
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    tui.wait_for("人格和记忆");
    tui.open_menu("人格");
    tui.wait_for("软件工程师");
    tui.key(b"\r");
    tui.wait_for("人设提醒短语");
    // 推演人生在第 8 行能改的（`FIELDS`），读到状态写「关」；关着的不露推演方法（2026-11-14 项目主人定）。
    let row = |t: &Tui, value: &str| {
        t.lines().iter().any(|l| {
            let words: Vec<&str> = l.split_whitespace().collect();
            words.windows(2).any(|w| w == ["推演人生", value])
        })
    };
    until(&mut tui, "推演人生 关", |t| row(t, "关"));
    assert!(!tui.shows("推演方法"), "{}", tui.lines().join("\n"));
    for _ in 0..7 {
        tui.key(b"j");
    }
    // 关着的先问一句，写清影响；停在「取消」上，取消了不开。
    tui.key(b"\r");
    tui.wait_for("开启推演人生");
    assert!(tui.shows("会花 token"), "{}", tui.lines().join("\n"));
    tui.key(b"\r");
    tui.pump(Duration::from_millis(500));
    assert!(row(&tui, "关"), "取消了不开：\n{}", tui.lines().join("\n"));
    // 再来一次，选「开启」。
    tui.key(b"\r");
    tui.wait_for("开启推演人生");
    tui.key(b"h");
    tui.key(b"\r");
    until(&mut tui, "推演人生 开", |t| row(t, "开"));
    // 开了才露出推演方法、人生会话的上下文窗口。
    assert!(
        tui.shows("推演方法") && tui.shows("人生会话上下文窗口"),
        "{}",
        tui.lines().join("\n")
    );
    // 开着的回车暂停，不问。
    tui.key(b"\r");
    until(&mut tui, "推演人生 暂停", |t| row(t, "暂停"));
    assert!(!tui.shows("开启推演人生"), "{}", tui.lines().join("\n"));
    assert!(
        !tui.shows("推演方法"),
        "暂停了收起：\n{}",
        tui.lines().join("\n")
    );
}
