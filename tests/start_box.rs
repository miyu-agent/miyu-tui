//! 伪终端里的端到端测试：新会话开头的人格、预设框开着时（蓝图 `tui.md`「新会话：人格、工作区」第 1c、2c 条）。框接管输入：
//! 打的字、粘贴的字哪都不进（2026-11-02 项目主人改：原来进藏着的输入框，看不见，选完一回车就发了出去），`a` 新建一个。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 等框关上（列表那一行不在了）。
fn closed(tui: &mut support::Tui, word: &str) {
    let end = Instant::now() + support::WAIT;
    while tui.shows(word) {
        assert!(Instant::now() < end, "框没关：\n{}", tui.lines().join("\n"));
        tui.pump(Duration::from_millis(50));
    }
}

#[test]
fn what_is_typed_while_the_pick_box_is_open_goes_nowhere() {
    let home = Home::new(Script::new([Play::Says("收到。")]));
    let mut tui = home.tui_with("zh_CN.UTF-8", &[("MIYU_TUI_PRESET", "")]);
    tui.wait_for("全部功能");
    // 框开着：粘贴一段，再打几个字。
    tui.send("\x1b[200~粘贴的话\x1b[201~".as_bytes());
    tui.type_text("xyz");
    tui.pump(Duration::from_millis(300));
    assert!(
        tui.shows("全部功能"),
        "框还开着：\n{}",
        tui.lines().join("\n")
    );
    tui.key(b"\r");
    closed(&mut tui, "全部功能");
    tui.pump(Duration::from_millis(200));
    assert!(
        !tui.shows("粘贴的话") && !tui.shows("xyz"),
        "框开着时打的字、粘贴的字不进输入框：\n{}",
        tui.lines().join("\n")
    );
    // 选了才能打字：照常发出去。
    tui.say("你好");
    tui.wait_for("收到。");
}

#[test]
fn a_in_the_pick_box_creates_one_and_comes_back_to_it() {
    let home = Home::new(Script::new([]));
    let mut tui = home.tui_with("zh_CN.UTF-8", &[("MIYU_TUI_PRESET", "")]);
    tui.wait_for("全部功能");
    // `a` 开配置页那个两格窗；id 留空让核心起，名字写好交。
    tui.type_text("a");
    tui.wait_for("新建预设");
    tui.key(b"\r");
    tui.type_text("框里建的");
    tui.key(b"\r");
    // 建好照配置页开它的详情窗（功能树），关了回框。
    tui.wait_for("基础系统");
    tui.key(b"\x1b");
    tui.wait_for("请选择预设");
    // 列表读回来：新的那一个在里面，光标停在它上面。
    let end = Instant::now() + support::WAIT;
    loop {
        let picked = tui
            .lines()
            .into_iter()
            .find(|l| l.contains('❯'))
            .unwrap_or_default();
        if picked.contains("框里建的") {
            break;
        }
        assert!(
            Instant::now() < end,
            "光标停在新建的那一个上：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(50));
    }
}
