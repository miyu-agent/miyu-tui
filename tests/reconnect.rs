//! 伪终端里的端到端测试：断线、重连一路（蓝图 `tui.md`「连核心」第 7 条，review B 批，2026-10-11）。核心停掉再起来，看
//! 界面接不接得上原来的会话、会话没了怎么办、断开期间错过的补不补得回来。
#![cfg(unix)]

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 断了以后一直重连，连不上时写核心没在跑。
const DOWN: &str = "核心未运行";

/// 当前会话在核心停着时被删了（review B3）：重连以后写「会话不存在」，上面的留着看；下一句话像按过 `/new` 一样开新
/// 会话。原来一直订阅它、一直失败，打字、`/new` 都没反应。
#[test]
fn a_session_deleted_while_the_core_was_away_says_so_and_the_next_sentence_starts_anew() {
    let home = Home::new(Script::new([
        Play::Says("第一句的回答。"),
        Play::Says("新会话的回答。"),
    ]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("第一句");
    tui.wait_for("第一句的回答。");
    home.stop_core();
    tui.wait_for(DOWN);
    std::fs::remove_dir_all(home.root().join("home/alice/sessions")).expect("删得掉");
    home.restart_core();
    tui.wait_for("会话不存在");
    assert!(tui.shows("第一句的回答。"), "上面的留着看");
    tui.say("第二句");
    tui.wait_for("新会话的回答。");
    assert!(
        !tui.shows("第一句的回答。"),
        "像按过 /new：换了一份空的正文\n{}",
        tui.lines().join("\n")
    );
}

/// `--resume` 一个不存在的会话：写「会话不存在」，不挑最近的那个；下一句话开新会话。
#[test]
fn resuming_a_session_that_does_not_exist_says_so_and_the_next_sentence_starts_anew() {
    let home = Home::new(Script::new([Play::Says("新会话的回答。")]));
    let missing = "01900000-0000-7000-8000-000000000000";
    let mut tui = home.tui_args("zh_CN.UTF-8", &["--resume", missing]);
    tui.wait_for("会话不存在");
    tui.say("你好");
    tui.wait_for("新会话的回答。");
}

/// 断开期间核心记下的事件，重连以后补回来（review B7：重连订阅原来不带 `after`）。核心重启时把在跑的后台命令记成
/// 「核心重启时停了」，这一条在重新订阅之前就写进了日志。
#[test]
fn what_the_core_wrote_while_reconnecting_is_caught_up() {
    let args = serde_json::json!({
        "command": "sleep 20", "description": "睡一会", "run_in_background": true
    })
    .to_string();
    let home = Home::with_tools(
        Script::new([
            Play::Calls(vec![("shell".into(), args)]),
            Play::Says("在后台跑了。"),
        ]),
        "",
    );
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("后台睡一会");
    // 测试的机器上没有沙盒：核心问要不要在沙盒外跑，允许本次。
    tui.wait_for("允许本次");
    tui.key(b"\r");
    tui.wait_for("在后台跑了。");
    tui.wait_for("1 个后台命令");
    home.stop_core();
    tui.wait_for(DOWN);
    home.restart_core();
    tui.wait_for("核心重启时已停止");
    assert!(!tui.shows("1 个后台命令"), "{}", tui.lines().join("\n"));
}

/// 切会话、新会话的历史还在读的时候（review B1）：回车不发，字留在输入框，弹「正在切换会话」；这时断了线，重连以后
/// 照样换上来。原来回车的话画进原来那个、发给了新的；断了线补发永远不结束，一直停在原来那个会话。
#[test]
fn a_switch_cut_off_by_the_core_going_away_lands_after_reconnecting() {
    let home = Home::new(Script::new([
        Play::Says("第 1 号的回答。"),
        Play::Says("第 2 号的回答。"),
    ]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("第 1 号的话");
    tui.wait_for("第 1 号的回答。");
    tui.say("/new");
    tui.pump(std::time::Duration::from_millis(200));
    tui.say("第 2 号的话");
    tui.wait_for("第 2 号的回答。");
    tui.say("/sessions");
    tui.wait_for("2 个");
    tui.key(b"\x1b[B");
    // 冻住核心再选：要第 1 号最新一页的请求停在路上。
    home.freeze_core();
    tui.key(b"\r");
    tui.pump(std::time::Duration::from_millis(300));
    tui.say("这时候说的");
    tui.wait_for("正在切换会话");
    assert!(tui.input().contains("这时候说的"), "{}", tui.input());
    home.stop_core();
    tui.wait_for(DOWN);
    home.restart_core();
    tui.wait_for("第 1 号的回答。");
    assert!(!tui.shows("第 2 号的回答。"), "{}", tui.lines().join("\n"));
}
