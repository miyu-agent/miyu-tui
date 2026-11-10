//! 太长的话（蓝图「输入框」第 11 条，施工 YC-11，压测报告 F5）：一两 MB 的粘贴回车，整段转成文字附件发出去、框清空；转不了的
//! （`!` 命令、超过附件上限）不发，字连块留在框里，弹一句多大。

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
fn pasting_two_megabytes_and_pressing_enter_sends_it_as_a_text_attachment() {
    let (mut app, mut sent) = crate::app::test_support::app();
    app.talk.transcript.session = Some("s1".into());
    let big = log();
    app.paste(&big);
    assert!(app.typing.input.editor.text().len() < 100, "收成了一块");
    while sent.try_recv().is_ok() {}
    press(&mut app, KeyCode::Enter);
    let mut got = None;
    while let Ok(command) = sent.try_recv() {
        if let Command::Send { text, pasted, .. } = command {
            got = Some((text, pasted));
        }
    }
    let (text, pasted) = got.expect("发了");
    let pasted = pasted.expect("带着文字附件");
    assert_eq!(text, "", "话里不带字");
    assert_eq!(pasted.text, big, "附件是原文");
    assert!(
        pasted.name.starts_with("pasted-") && pasted.name.ends_with(".txt"),
        "{}",
        pasted.name
    );
    assert_eq!(app.typing.input.editor.text(), "", "框清空");
    let said = app.talk.transcript.entries.last().expect("正文里有这一句");
    assert_eq!(said.text, format!("[{}]", pasted.name), "正文里照附件写");
    assert!(
        said.pasted
            .iter()
            .all(|c| c.attachment() && c.full.len() < 100),
        "块里不放原文"
    );
    let notice = app.typing.notice.as_ref().expect("弹了一句");
    assert!(notice.text.contains(&pasted.name), "{}", notice.text);
}

#[test]
fn over_the_attachment_limit_it_stays_in_the_box() {
    let (mut app, mut sent) = crate::app::test_support::app();
    app.talk.transcript.session = Some("s1".into());
    app.config.attachments.max_mib = 1;
    let big = log();
    app.paste(&big);
    let box_text = app.typing.input.editor.text().to_string();
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
        went |= matches!(command, Command::Send { text, pasted: None, .. } if text == "短的");
    }
    assert!(went, "短的发出去了");
}

#[test]
fn the_name_is_like_the_web_heads() {
    let at: jiff::Zoned = "2026-11-10T09:05:03[Asia/Tokyo]".parse().unwrap();
    assert_eq!(crate::core::pasted_name(&at), "pasted-20261110-090503.txt");
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
