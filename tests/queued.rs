//! 排着队的话（蓝图 `tui.md`「运行状态行和排队的消息」第 5 条）：她跑工具的时候回车发的，下一次请求带上，进正文只出一次，
//! 后面的轮不再冒出来（2026-10-26 项目主人报：步与步之间发的、重开以后的，后面的轮又画了一遍）。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 屏上 `text` 出现几次、第一次在第几行。
fn spots(tui: &Tui, text: &str) -> (usize, Option<usize>) {
    let lines = tui.lines();
    let count = lines.iter().filter(|l| l.contains(text)).count();
    (count, lines.iter().position(|l| l.contains(text)))
}

// 前台 `sleep` 一段到 2 秒的核心整条拒（核心 62b573846，反馈第 8 条），这里睡 1.5 秒。
#[test]
fn a_message_sent_between_steps_is_drawn_once_where_she_heard_it() {
    let args = serde_json::json!({"command": "sleep 1.5", "description": "睡一会"}).to_string();
    let script = Script::new([
        Play::Calls(vec![("shell".into(), args)]),
        Play::Says("看到插的那句了。"),
        Play::Says("第二轮好了。"),
    ]);
    let home = Home::with_tools(script, "[permission]\nlevel = \"full\"\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("第一句");
    tui.wait_for("睡一会");
    // 工具还在跑：普通回车发出去，排着。
    tui.say("插一句");
    tui.wait_for("看到插的那句了。");
    let (count, at) = spots(&tui, "插一句");
    let (_, reply) = spots(&tui, "看到插的那句了。");
    assert_eq!(count, 1, "只出一次：\n{}", tui.lines().join("\n"));
    assert!(
        at < reply,
        "排在听到它的回答前面：\n{}",
        tui.lines().join("\n")
    );
    tui.say("第二句");
    tui.wait_for("第二轮好了。");
    tui.pump(Duration::from_millis(300));
    let (count, at) = spots(&tui, "插一句");
    let (_, second) = spots(&tui, "第二句");
    assert_eq!(count, 1, "下一轮不再冒出来：\n{}", tui.lines().join("\n"));
    assert!(at < second, "还在原处：\n{}", tui.lines().join("\n"));
}

/// 测试家目录里唯一的那个会话的编号。
fn only_session(home: &Home) -> String {
    let dir = home.root().join("home/alice/sessions");
    let ids: Vec<String> = std::fs::read_dir(dir)
        .expect("会话目录")
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect();
    assert_eq!(ids.len(), 1, "{ids:?}");
    ids[0].clone()
}

#[test]
fn after_reopening_a_message_sent_between_steps_stays_where_she_heard_it() {
    // 重开以后核心照日志补发，流式的增量不补：听到没听到照落盘的 `model.called` 认（第二张截图：18 小时前排过的话又画成了排队）。
    let args = serde_json::json!({"command": "sleep 1.5", "description": "睡一会"}).to_string();
    let script = Script::new([
        Play::Calls(vec![("shell".into(), args)]),
        Play::Thinks {
            thinking: "想一想",
            text: "看到插的那句了。",
        },
        Play::Says("第二轮好了。"),
    ]);
    let home = Home::with_tools(script, "[permission]\nlevel = \"full\"\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("第一句");
    tui.wait_for("睡一会");
    tui.say("插一句");
    tui.wait_for("看到插的那句了。");
    drop(tui);
    let id = only_session(&home);
    let mut tui = home.tui_args("zh_CN.UTF-8", &["--resume", &id]);
    tui.wait_for("看到插的那句了。");
    tui.pump(Duration::from_millis(500));
    let (count, at) = spots(&tui, "插一句");
    let (_, reply) = spots(&tui, "看到插的那句了。");
    assert_eq!(count, 1, "只出一次：\n{}", tui.lines().join("\n"));
    assert!(at < reply, "排在回答前面：\n{}", tui.lines().join("\n"));
    tui.say("第二句");
    tui.wait_for("第二轮好了。");
    tui.pump(Duration::from_millis(300));
    let (count, at) = spots(&tui, "插一句");
    let (_, second) = spots(&tui, "第二句");
    assert_eq!(count, 1, "下一轮不再冒出来：\n{}", tui.lines().join("\n"));
    assert!(at < second, "还在原处：\n{}", tui.lines().join("\n"));
}
