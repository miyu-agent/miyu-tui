//! 终端的回话从按键里截下来（`replies.rs`）。

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use super::{Read, Replies};

fn key(c: char, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(KeyCode::Char(c), modifiers))
}

/// crossterm 读 `bytes` 会读成的那一串：`ESC x` 是 Alt+x，BEL 是 Ctrl+G，别的照字。
fn typed(bytes: &str) -> Vec<Event> {
    let mut out = Vec::new();
    let mut chars = bytes.chars();
    while let Some(c) = chars.next() {
        out.push(match c {
            '\x1b' => key(chars.next().unwrap(), KeyModifiers::ALT),
            '\x07' => key('g', KeyModifiers::CONTROL),
            c => key(c, KeyModifiers::NONE),
        });
    }
    out
}

fn feed(replies: &mut Replies, events: Vec<Event>) -> Vec<Read> {
    events.into_iter().flat_map(|e| replies.feed(e)).collect()
}

#[test]
fn a_reply_from_the_terminal_is_taken_whole_and_never_typed() {
    // 2026-10-27 伪终端实测：kitty 点通知回的这一串原来进了输入框。
    let mut replies = Replies::default();
    let got = feed(&mut replies, typed("\x1b]99;i=miyu-s-01a1;\x1b\\"));
    assert_eq!(got, [Read::Reply("99;i=miyu-s-01a1;".into())]);
    assert!(!replies.holding());
    // BEL 收尾的一样。
    let got = feed(&mut replies, typed("\x1b]11;rgb:1a1a/1b1b/2626\x07"));
    assert_eq!(got, [Read::Reply("11;rgb:1a1a/1b1b/2626".into())]);
}

#[test]
fn alt_bracket_pressed_by_a_person_goes_through_as_it_was() {
    let mut replies = Replies::default();
    // 人按的 Alt+]：攒着，等不到下一个就照原样交回。
    let got = feed(&mut replies, typed("\x1b]"));
    assert!(got.is_empty() && replies.holding());
    assert_eq!(replies.expire(), [Read::Event(key(']', KeyModifiers::ALT))]);
    // 后面跟着的不是字（方向键这类）：攒着的连它一起照原样交回。
    let mut events = typed("\x1b]a");
    events.push(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)));
    let got = feed(&mut replies, events.clone());
    assert_eq!(got, events.into_iter().map(Read::Event).collect::<Vec<_>>());
    assert!(!replies.holding());
    // 平常打的字直接交回。
    let got = feed(&mut replies, typed("ab"));
    assert_eq!(
        got,
        typed("ab").into_iter().map(Read::Event).collect::<Vec<_>>()
    );
}
