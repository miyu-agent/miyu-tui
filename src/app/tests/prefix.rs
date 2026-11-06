//! 前缀要变时问人（蓝图「前缀要变」，排期第 59 条）：开框停在「直接发送」，`←` `→` 挪到头停，`Enter` 按；前两个照记着的那一条
//! 再发（`Command::Retry`），取消、`Esc` 不再发，没发出去的那句放回框里；点按钮就是按。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Position, Rect};

use super::app;
use crate::app::Panel;
use crate::core::{Command, Update};
use crate::input::Draft;

fn press(app: &mut crate::app::App, code: KeyCode) {
    app.key(KeyEvent::new(code, KeyModifiers::NONE));
}

/// 会话开着（开头选人格、预设的那一道不拦按键）。
fn opened() -> (
    crate::app::App,
    tokio::sync::mpsc::UnboundedReceiver<Command>,
) {
    let (mut app, sent) = app();
    app.talk.transcript.session = Some("s1".into());
    (app, sent)
}

fn asked() -> Update {
    Update::PrefixChanging {
        files: vec!["root/personas/alpha/prompts/persona.md".into()],
        more: 0,
        command: None,
    }
}

#[test]
fn the_buttons_move_and_the_middle_one_retries_with_compact() {
    let (mut app, mut sent) = opened();
    app.core(asked());
    assert_eq!(app.boxes.panel, Some(Panel::Prefix { button: 0 }));
    assert_eq!(app.talk.prefix_ask.as_ref().map(|a| a.files.len()), Some(1));
    for code in [KeyCode::Right, KeyCode::Right, KeyCode::Right] {
        press(&mut app, code);
    }
    assert_eq!(app.boxes.panel, Some(Panel::Prefix { button: 2 }), "到头停");
    press(&mut app, KeyCode::Left);
    while sent.try_recv().is_ok() {}
    press(&mut app, KeyCode::Enter);
    let commands: Vec<Command> = std::iter::from_fn(|| sent.try_recv().ok()).collect();
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, Command::Retry("compact"))),
        "{commands:?}"
    );
    assert_eq!(app.boxes.panel, None);
    assert!(app.talk.prefix_ask.is_none());
}

#[test]
fn cancel_sends_nothing_and_puts_the_words_back() {
    let (mut app, mut sent) = opened();
    app.talk.unsent = Some(crate::app::Unsent {
        draft: Some(Draft::plain("看看改了什么")),
        original: None,
        turn: None,
    });
    app.core(asked());
    while sent.try_recv().is_ok() {}
    press(&mut app, KeyCode::Esc);
    let commands: Vec<Command> = std::iter::from_fn(|| sent.try_recv().ok()).collect();
    assert!(
        !commands.iter().any(|c| matches!(c, Command::Retry(_))),
        "{commands:?}"
    );
    assert_eq!(app.boxes.panel, None);
    assert_eq!(app.typing.input.editor.text(), "看看改了什么");
}

#[test]
fn clicking_a_button_presses_it() {
    let (mut app, mut sent) = opened();
    app.core(asked());
    // 框画在这里：按钮在第 5 行（`ui/prefix_ask.rs`），行首空两格。
    app.screen.areas.menu = Rect::new(0, 10, 60, 7);
    app.screen.areas.menu_text = Rect::new(1, 11, 58, 5);
    app.screen.panel_rows = vec![None, None, None, None, Some(0)];
    while sent.try_recv().is_ok() {}
    // 「[ 直接发送 ]」从第 3 列起占 14 列。
    assert!(app.prefix_click(Position::new(4, 15), true));
    let commands: Vec<Command> = std::iter::from_fn(|| sent.try_recv().ok()).collect();
    assert!(
        commands.iter().any(|c| matches!(c, Command::Retry("keep"))),
        "{commands:?}"
    );
}
