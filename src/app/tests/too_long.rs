//! 太长的话不发（蓝图「输入框」第 11 条，施工 YC-11，压测报告 F5）：一两 MB 的粘贴回车，不发请求，字连块留在框里，弹一句多大。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::core::Command;

/// 两 MB 上下的一段日志。
fn log() -> String {
    "2026-11-10 10:00:00 ERROR 连不上 upstream\n".repeat(50_000)
}

/// 照真按键走：回车交回的「发」也办（`handle`）。
fn press(app: &mut crate::app::App, code: KeyCode) {
    app.handle(ratatui::crossterm::event::Event::Key(KeyEvent::new(
        code,
        KeyModifiers::NONE,
    )));
}

/// 回车以后没发 `session.send`、框里还是那一块、提示写了多大。
fn kept(
    app: &crate::app::App,
    sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>,
    box_text: &str,
    big: &str,
) {
    while let Ok(command) = sent.try_recv() {
        assert!(!matches!(command, Command::Send { .. }), "没发");
    }
    assert_eq!(app.typing.input.editor.text(), box_text, "框里照旧是那一块");
    assert_eq!(app.typing.input.draft().expand(), big, "块里是原文");
    let notice = app.typing.notice.as_ref().expect("弹了一句");
    assert!(notice.text.contains("MB"), "{}", notice.text);
}

#[test]
fn pasting_two_megabytes_and_pressing_enter_keeps_it_and_says_how_big() {
    let (mut app, mut sent) = crate::app::test_support::app();
    app.talk.transcript.session = Some("s1".into());
    let big = log();
    app.paste(&big);
    let box_text = app.typing.input.editor.text().to_string();
    assert!(box_text.len() < 100, "收成了一块：{box_text}");
    press(&mut app, KeyCode::Enter);
    kept(&app, &mut sent, &box_text, &big);
    // 删短了照常发。
    app.typing
        .input
        .editor
        .set_draft(crate::input::Draft::plain("短的"));
    press(&mut app, KeyCode::Enter);
    let mut went = false;
    while let Ok(command) = sent.try_recv() {
        went |= matches!(command, Command::Send { text, .. } if text == "短的");
    }
    assert!(went, "短的发出去了");
}

#[test]
fn a_shell_command_that_long_is_kept_too() {
    let (mut app, mut sent) = crate::app::test_support::app();
    app.talk.transcript.session = Some("s1".into());
    press(&mut app, KeyCode::Char('!'));
    assert!(app.typing.input.shell(), "进了跑命令的模式");
    let big = log();
    app.paste(&big);
    let box_text = app.typing.input.editor.text().to_string();
    press(&mut app, KeyCode::Enter);
    kept(&app, &mut sent, &box_text, &big);
}
