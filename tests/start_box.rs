//! 伪终端里的端到端测试：新会话开头的人格、预设框开着时打的字、粘贴的字（蓝图 `tui.md`「新会话：人格、工作区」第 1c 条，
//! 2026-10-26 项目主人报：刚启动马上发的第一条长消息有时被吞）。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn what_is_typed_while_the_pick_box_is_open_waits_in_the_input_box() {
    let home = Home::new(Script::new([Play::Says("收到长消息。")]));
    let mut tui = home.tui_with("zh_CN.UTF-8", &[("MIYU_TUI_PRESET", "")]);
    tui.wait_for("全部功能");
    // 框开着：粘贴一段，再打几个字（打了字以后 j、k 也当字）。
    tui.send("\x1b[200~第一句很长的话\x1b[201~".as_bytes());
    tui.type_text(" jk ok");
    tui.pump(Duration::from_millis(300));
    assert!(
        tui.shows("全部功能"),
        "框还开着：\n{}",
        tui.lines().join("\n")
    );
    // 回车照旧选框里的：框关了，字还在输入框里。
    tui.key(b"\r");
    let end = Instant::now() + support::WAIT;
    while tui.shows("全部功能") {
        assert!(Instant::now() < end, "框没关：\n{}", tui.lines().join("\n"));
        tui.pump(Duration::from_millis(50));
    }
    tui.wait_for("第一句很长的话 jk ok");
    // 再回车发出去。
    tui.key(b"\r");
    tui.wait_for("收到长消息。");
}

/// 新会话开头的人格、预设框（列表没读到时写「正在加载…」的那个也算）。
const PICK_WORDS: [&str; 3] = ["请选择人格", "请选择预设", "正在加载…"];

#[test]
fn starting_in_the_most_recent_session_asks_for_no_persona_or_preset() {
    // 2026-10-26 项目主人：启动时打开的是最近那个会话，不该再弹人格、预设框（那个会话已经选过了）。真机冷启动时核心起来
    // 要一会儿，连上以前照记下的列表先开了框，会话接上了也没收；这里冻住核心造出这一段。
    let home = Home::with_settings(
        Script::new([Play::Says("以前的回答。")]),
        "[ui]\nstartup = \"recent\"\n",
    );
    home.persona("pirate", "[persona]\nname = { zh = \"海盗\" }\n", None);
    // 头一回照常弹框选（人格、预设列表记进起来时先照着画的那一份，`look.rs`）。
    let ask = [("MIYU_TUI_PERSONA", ""), ("MIYU_TUI_PRESET", "")];
    let mut first = home.tui_with("zh_CN.UTF-8", &ask);
    first.wait_for("请选择人格");
    first.key(b"\r");
    first.wait_for("请选择预设");
    first.key(b"\r");
    first.wait_for("工作区");
    first.say("以前说的话");
    first.wait_for("以前的回答。");
    drop(first);
    // 第二回：上一回没进已有的会话，连上以前照记下的开框；连上进了最近那个，框收掉。
    home.freeze_core();
    let mut again = home.tui_with("zh_CN.UTF-8", &ask);
    again.pump(Duration::from_millis(800));
    home.stop_core();
    home.restart_core();
    again.wait_for("以前的回答。");
    again.pump(Duration::from_millis(500));
    for word in PICK_WORDS {
        assert!(
            !again.shows(word),
            "会话接上了，框收掉：\n{}",
            again.lines().join("\n")
        );
    }
    drop(again);
    // 第三回：记着上一回进了已有的会话，连上以前也不开框，照常画输入框（列表还在读的「正在加载…」那个框也不画）。
    home.freeze_core();
    let mut third = home.tui_with("zh_CN.UTF-8", &ask);
    third.wait_for("❯");
    third.pump(Duration::from_millis(800));
    for word in PICK_WORDS {
        assert!(
            !third.shows(word),
            "连上以前不开框：\n{}",
            third.lines().join("\n")
        );
    }
    home.stop_core();
    home.restart_core();
    third.wait_for("以前的回答。");
    third.pump(Duration::from_millis(500));
    for word in PICK_WORDS {
        assert!(
            !third.shows(word),
            "接上了也不开框：\n{}",
            third.lines().join("\n")
        );
    }
}
