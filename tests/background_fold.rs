//! 后台跑完、时间线收起以后切回来（蓝图 `tui.md`「会话列表 `/sessions`」第 5 条）：伪终端里的真界面连一份照剧本回话的
//! 核心。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::Home;

fn shell(args: serde_json::Value) -> (String, String) {
    ("shell".into(), args.to_string())
}

#[test]
fn a_session_that_folded_in_the_background_comes_back_without_a_blank_screen() {
    // 2026-11-16 项目主人报：交代一件调很多工具的活，`/new` 切走让它在后台跑，跑完时间线收起，切回来正文只剩最底下一行，
    // 上面一大片空白。还有后台命令在跑的会话切走以后照旧停放着（「会话列表」第 4 条），视口也跟着停放：切走那一刻
    // 「只往下走」记下的底边是展开着的几十步，收起以后内容只剩十来行。
    let mut calls: Vec<_> = (1..=25)
        .map(|i| {
            let command = format!("seq 1 40 | sed 's/^/批{i}行/'");
            shell(serde_json::json!({"command": command, "description": format!("第{i}批")}))
        })
        .collect();
    calls.push(shell(serde_json::json!({
        "command": "sleep 30",
        "run_in_background": true,
        "description": "后台睡",
    })));
    let script = Script::new([
        Play::Calls(calls),
        Play::Late {
            ms: 2000,
            text: "后台做完了。",
        },
    ]);
    // 进行中整段展开、收起时放开视口（项目主人自己的设置）：切走那一刻正文远长过一屏。
    let settings = "[permission]\nlevel = \"full\"\n[tui]\ntimeline_limit_live = false\n";
    let home = Home::with_tools(script, settings);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("跑很多命令");
    tui.wait_for("后台睡");
    tui.pump(Duration::from_millis(300));
    tui.say("/new");
    tui.pump(Duration::from_millis(300));
    // 列表里那一行不再转圈、写上多久以前：这一轮在后台做完了。
    let until = Instant::now() + support::WAIT;
    loop {
        tui.say("/sessions");
        tui.wait_for("1 个");
        if tui.shows("刚才") {
            break;
        }
        assert!(
            Instant::now() < until,
            "后台那一轮做不完：\n{}",
            tui.lines().join("\n")
        );
        tui.key(b"\x1b");
        tui.pump(Duration::from_millis(500));
    }
    tui.pump(Duration::from_millis(300));
    tui.key(b"\r");
    tui.wait_for_done();
    tui.pump(Duration::from_millis(300));
    assert!(
        tui.shows("┃ 跑很多命令") && tui.shows("后台做完了。"),
        "收起以后一屏放得下，从头露出来，不留空白：\n{}",
        tui.lines().join("\n")
    );
}
