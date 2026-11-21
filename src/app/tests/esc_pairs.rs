//! 两下 `Esc` 打断只数归输入框的那几下（蓝图 `tui.md`「按键」`Esc`，ACC-1121-#1）：关列表、取消选区的那一下被别处收了，
//! 不算第一下，也把记着的上一下作废；发出去一句也作废。2026-11-20 主人本机：发出去一句以后接连按 `Esc`，这一轮被打断、
//! 撤掉，接着打的 `/loop` 因为有能恢复的撤销一直只记下、不开轮。

use std::time::Instant;

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::app::App;
use crate::app::test_support::app;
use crate::core::Command;
use crate::transcript::Link;

fn press(app: &mut App, code: KeyCode) {
    app.handle(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
}

/// 在回答的界面，连着核心、看着一个会话。
fn answering() -> (App, UnboundedReceiver<Command>) {
    let (mut app, sent) = app();
    app.talk.transcript.link = Link::Ready;
    app.talk.transcript.session = Some("s1".into());
    app.talk.transcript.running = Some(Instant::now());
    (app, sent)
}

/// 到现在为止发出去的打断。
fn interrupts(sent: &mut UnboundedReceiver<Command>) -> usize {
    let mut n = 0;
    while let Ok(command) = sent.try_recv() {
        n += usize::from(matches!(command, Command::Interrupt { .. }));
    }
    n
}

#[test]
fn an_escape_that_closes_the_command_list_breaks_the_pair() {
    let (mut app, mut sent) = answering();
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('/'));
    assert!(app.menu_matches().is_some(), "打 `/` 开了命令列表");
    press(&mut app, KeyCode::Esc);
    assert!(app.menu_matches().is_none(), "这一下关了列表");
    press(&mut app, KeyCode::Esc);
    assert_eq!(
        interrupts(&mut sent),
        0,
        "关列表那一下不算，后一下只是第一下"
    );
    press(&mut app, KeyCode::Esc);
    assert_eq!(interrupts(&mut sent), 1, "接着再按一下才打断");
}

#[test]
fn an_escape_that_clears_a_sidebar_selection_is_not_a_first_press() {
    let (mut app, mut sent) = answering();
    app.screen.side_select.press(1, 1);
    app.screen.side_select.drag(5, 1);
    press(&mut app, KeyCode::Esc);
    assert!(!app.screen.side_select.active(), "这一下取消侧边栏的选区");
    press(&mut app, KeyCode::Esc);
    assert_eq!(interrupts(&mut sent), 0, "取消选区那一下不算");
}

#[test]
fn sending_a_line_forgets_the_first_escape() {
    let (mut app, mut sent) = answering();
    press(&mut app, KeyCode::Esc);
    for c in "ok".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    press(&mut app, KeyCode::Enter);
    assert!(
        std::iter::from_fn(|| sent.try_recv().ok()).any(|c| matches!(c, Command::Send { .. })),
        "排着发出去了"
    );
    press(&mut app, KeyCode::Esc);
    assert_eq!(interrupts(&mut sent), 0, "发出去以后按的是新的第一下");
}
