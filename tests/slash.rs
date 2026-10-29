//! 核心认的斜杠命令（蓝图 `tui.md`「斜杠命令列表」第 4、6 条，核心 X-14）：家目录里放的命令模板照 `command.catalog`
//! 列进命令列表（名字后面暗色跟参数提示、说明照界面语言），打了整行交给核心；正文里是人敲的那一行，模型看到的是展开的
//! 那段。终端不认的名字也交给核心，被拒的弹核心的原话、字放回输入框。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 一份命令模板：说明、中文说明、参数提示、正文里接参数。
const REVIEW: &str = "---\ndescription: Review the code\nargument-hint: <路径>\nmetadata:\n  summary_zh: 审一遍代码\n---\n请把 $ARGUMENTS 这里的代码从头到尾审一遍。\n";

#[test]
fn a_template_command_is_listed_and_shows_the_typed_line() {
    let script = Script::new([Play::Says("看过了。")]);
    let home = Home::new(script.clone());
    let commands = home.root().join("commands");
    std::fs::create_dir_all(&commands).expect("建得了");
    std::fs::write(commands.join("review.md"), REVIEW).expect("写得进去");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    // 列表里有它：名字后面暗色跟着参数提示，说明照中文。
    tui.type_text("/revi");
    tui.wait_for("/review <路径>");
    assert!(tui.shows("审一遍代码"), "{}", tui.lines().join("\n"));
    // 接着打参数、回车：交给核心展开，正文里是打的那一行。
    tui.type_text("ew src/lib.rs");
    tui.key(b"\r");
    tui.wait_for("看过了。");
    let screen = tui.lines().join("\n");
    assert!(
        screen.contains("/review src/lib.rs"),
        "正文里是人敲的那一行：\n{screen}"
    );
    assert!(
        !screen.contains("从头到尾审一遍"),
        "展开的那段不给人看：\n{screen}"
    );
    let sent = format!("{:?}", script.requests().last());
    assert!(
        sent.contains("请把 src/lib.rs 这里的代码从头到尾审一遍"),
        "模型看到展开的那段：{sent}"
    );
}

#[test]
fn a_command_no_one_knows_is_refused_by_the_core_and_not_put_back() {
    let home = Home::new(Script::new([]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    tui.type_text("/nosuchthing");
    tui.key(b"\x1b");
    tui.key(b"\r");
    tui.pump(Duration::from_millis(500));
    // 弹核心说的那一句（`unknown_command` 的原话恰好也是「命令不存在」；终端自己的那句字已经去掉了）；打的字不回到输入框
    // （2026-10-23 项目主人：「输入不存在的命令不应该把命令再回填到输入框」）。
    assert!(tui.shows("命令不存在"), "{}", tui.lines().join("\n"));
    assert!(
        !tui.input().contains("/nosuchthing"),
        "输入框空着（只有提示）：{}",
        tui.lines().join("\n")
    );
}

#[test]
fn up_and_ctrl_k_move_the_command_list_and_never_recall_input_history() {
    // 2026-10-20 项目主人：「命令历史可以向下翻页但是不能向上翻页了，向上会把输入 prompt 历史调用出来」「Ctrl+K 也是掉历史」：
    // 列表开着时 ↑、Ctrl+K 只在列表里往上挪。
    let home = Home::new(Script::new([Play::Says("好。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("历史里的一句");
    tui.wait_for("好。");
    tui.type_text("/c");
    tui.wait_for("命令");
    let picked = |tui: &support::Tui| {
        tui.lines()
            .into_iter()
            .find(|l| l.contains("❯ /"))
            .unwrap_or_default()
    };
    let first = picked(&tui);
    for up in [&b"\x1b[A"[..], &b"\x0b"[..]] {
        tui.key(b"\x1b[B");
        tui.pump(std::time::Duration::from_millis(100));
        assert_ne!(picked(&tui), first, "往下挪了");
        tui.key(up);
        tui.pump(std::time::Duration::from_millis(100));
        assert_eq!(
            picked(&tui),
            first,
            "往上挪回来：\n{}",
            tui.lines().join("\n")
        );
        assert!(
            !tui.shows("❯ 历史里的一句"),
            "输入框里没换成历史里的那一句：\n{}",
            tui.lines().join("\n")
        );
    }
}

#[test]
fn loop_from_the_core_is_listed_and_taken_by_the_core() {
    // 核心 J-2：`/loop` 是核心的命令，照 `command.catalog` 列出来；打了整行交给 `command.run`，核心收下（2026-10-27 项目主人：
    // 终端只做 loop）。工具面上要有 `schedule_wakeup`：测具登记出厂的工具。
    let home = Home::with_tools(Script::new([]), "");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.type_text("/lo");
    tui.wait_for("/loop [<间隔>] <要说的话> | off");
    tui.key(b"\x7f\x7f\x7f");
    let start = tui.record_from_here();
    tui.say("/loop 10m 看一眼 CI");
    tui.pump(Duration::from_millis(1500));
    let frames = support::frames(start, &tui.recorded());
    assert!(
        frames
            .iter()
            .all(|f| !f.iter().any(|l| l.contains("命令不存在"))),
        "核心收下了，不弹「命令不存在」"
    );
    assert!(!tui.input().contains("/loop"), "交出去了：{}", tui.input());
}
