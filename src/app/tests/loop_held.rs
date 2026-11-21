//! 循环这一次只记下没开轮（`held`）的提示（蓝图 `tui.md`「循环」第 1 条，ACC-1121-#1）：`/loop` 收下以后交的第一次，核心自己的回执
//! 已经说了循环先不跑（核心 FX-12），终端不再另写「循环这一轮没开始…」；之后到点、没有回执的那几次照旧写（2026-11-21 core1 定）。

use crate::core::{Loop, LoopChange, LoopEvery, Update};
use crate::transcript::Link;

fn round(last: Option<&str>) -> Loop {
    Loop {
        id: "t7".into(),
        session: "s1".into(),
        every: LoopEvery::Seconds(600),
        next: None,
        paused: false,
        prompt: "看一眼 CI".into(),
        last: last.map(|at| (at.to_string(), "held".to_string())),
    }
}

fn changed(app: &mut crate::app::App, last: Option<&str>) {
    let update = Update::Loops(LoopChange::One("t7".into(), Some(round(last))));
    app.core(update);
}

fn told(app: &crate::app::App) -> bool {
    let held = &app.config.text.loops.held;
    app.typing
        .notice
        .as_ref()
        .is_some_and(|n| n.text.contains(held.as_str()))
}

#[test]
fn the_first_held_round_after_slash_loop_leaves_it_to_the_cores_receipt() {
    let (mut app, _sent) = crate::app::test_support::app();
    app.talk.transcript.link = Link::Ready;
    app.talk.transcript.session = Some("s1".into());
    app.slash("/loop 10m 看一眼 CI");
    // 新的一行到了，紧接着第一次交了、只记下：回执会说，这里不写。
    changed(&mut app, None);
    changed(&mut app, Some("2026-11-21T10:00:00.000Z"));
    assert!(!told(&app), "第一次照核心的回执说");
    // 十分钟以后到点又只记下：没有回执，照旧写。
    app.typing.notice = None;
    changed(&mut app, Some("2026-11-21T10:10:00.000Z"));
    assert!(told(&app), "之后的照旧提示");
}
