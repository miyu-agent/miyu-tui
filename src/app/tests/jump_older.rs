//! `/jump` 开着、更早的页还没读上来时连按 `↑`（蓝图 `tui.md`「跳到说过的话」第 2 条）：多出列表的那几下记着，页读上来以后
//! 接着往上走，按了几下就停在第几句；原来停在读进来的最早那一句，多按的丢了（LiteMiyu LM-13 读代码判的，这里查实）。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use super::app;
use crate::app::App;
use crate::core::{Push, Update, read_replayed};

const AT: &str = "2026-11-20T08:00:00Z";

/// 第 `n` 句你说的话和接着的一轮，事件编号从 `n * 10` 起。
fn turn(n: u64) -> Vec<Value> {
    let seq = n * 10;
    vec![
        json!({"seq": seq, "at": AT, "kind": "message.user", "by": {"kind": "person"},
               "body": {"blocks": [{"type": "text", "text": format!("第 {n} 句")}]}}),
        json!({"seq": seq + 1, "at": AT, "kind": "turn.started", "turn": seq + 1,
               "body": {"trigger": seq}}),
        json!({"seq": seq + 2, "at": AT, "kind": "turn.ended", "turn": seq + 1,
               "body": {"reason": "completed"}}),
    ]
}

fn page(said: std::ops::RangeInclusive<u64>) -> Vec<Push> {
    let events: Vec<Value> = said.flat_map(turn).collect();
    read_replayed(&events)
}

fn to(app: &mut App, update: Update) {
    app.core(Update::Elsewhere {
        session: "s1".into(),
        update: Box::new(update),
    });
}

/// 进已有的会话 `s1`：最新一页是第 11～13 句，更早的还有。
fn resumed() -> App {
    let (mut app, _sent) = app();
    app.core(Update::Ready("s1".into()));
    for push in page(11..=13) {
        to(&mut app, Update::Push(push));
    }
    to(&mut app, Update::Paged(true));
    app
}

fn older(app: &mut App, said: std::ops::RangeInclusive<u64>, more: bool) {
    let pushes = page(said);
    to(
        app,
        Update::Older {
            pushes,
            more,
            failed: None,
        },
    );
}

fn press(app: &mut App, code: KeyCode, times: usize) {
    for _ in 0..times {
        app.history_key(KeyEvent::new(code, KeyModifiers::NONE));
    }
}

/// 列表里选中的那一句。
fn selected(app: &App) -> String {
    let (list, _) = app.list_items();
    let found = app.typing.history.matches(&list);
    found[app.typing.history.selected].draft.text.clone()
}

#[test]
fn ups_pressed_before_the_older_page_arrives_still_count() {
    let mut app = resumed();
    app.open_jump();
    assert!(app.typing.history.open && app.typing.history.jump);
    press(&mut app, KeyCode::Up, 5);
    assert_eq!(
        selected(&app),
        "第 11 句",
        "页还没到：停在读进来的最早那一句"
    );
    older(&mut app, 1..=10, false);
    // 最新的是第 13 句（0），按了 5 下：第 8 句。
    assert_eq!(selected(&app), "第 8 句", "页到了以后接着走完按的那几下");
}

#[test]
fn the_owed_ups_carry_over_pages_and_stop_at_the_first_thing_said() {
    let mut app = resumed();
    app.open_jump();
    press(&mut app, KeyCode::Up, 20);
    older(&mut app, 8..=10, true);
    assert_eq!(selected(&app), "第 8 句", "这一页走到头，还欠的等下一页");
    older(&mut app, 1..=7, false);
    assert_eq!(selected(&app), "第 1 句", "会话没那么长：停在最早那一句");
    press(&mut app, KeyCode::Down, 1);
    assert_eq!(
        selected(&app),
        "第 2 句",
        "读完了不再欠：按一下 ↓ 就往回走一句"
    );
}

#[test]
fn a_down_takes_back_an_owed_up_and_typing_forgets_them() {
    let mut app = resumed();
    app.open_jump();
    press(&mut app, KeyCode::Up, 5);
    press(&mut app, KeyCode::Down, 2);
    older(&mut app, 1..=10, true);
    assert_eq!(selected(&app), "第 10 句", "按了 5 上 2 下：等于往上 3 下");

    let mut app = resumed();
    app.open_jump();
    press(&mut app, KeyCode::Up, 5);
    press(&mut app, KeyCode::Char('句'), 1);
    older(&mut app, 1..=10, true);
    assert_eq!(
        app.typing.history.selected, 0,
        "打了字：选中回到最新的，欠的不要了"
    );
}
