//! 跟着会话走的那一份（发版前 check 第 17 条）：`Ctrl+C` 清掉的、`Ctrl+S` 暂存的、跑命令的模式、翻历史翻到哪，切走时交出去，
//! 换上别的会话的就没有，切回来放回去。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::input::{Desk, Draft, InputBox};

fn press(i: &mut InputBox, code: KeyCode, modifiers: KeyModifiers) {
    i.key(KeyEvent::new(code, modifiers));
}

#[test]
fn the_cleared_text_comes_back_only_in_its_own_session() {
    let mut i = super::input(40);
    i.editor.insert("写给一号会话的");
    press(&mut i, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert!(i.editor.is_empty());
    let one = i.take_desk();
    assert!(!one.is_empty(), "清掉的那一份记在一号名下");
    i.put_desk(Desk::default());
    press(&mut i, KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(i.editor.text(), "", "别的会话按 ↑ 找不回一号清掉的");
    assert!(i.take_desk().is_empty());
    i.put_desk(one);
    press(&mut i, KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(i.editor.text(), "写给一号会话的", "切回来按 ↑ 找回");
}

#[test]
fn the_stash_the_shell_mode_and_browsing_stay_with_their_session() {
    let mut i = super::input(40);
    i.remember(Draft::plain("早先发过的"));
    i.editor.insert("暂存的");
    press(&mut i, KeyCode::Char('s'), KeyModifiers::CONTROL);
    i.editor.insert("!ls");
    i.absorb_bang();
    assert!(i.shell() && i.stashed());
    let one = i.take_desk();
    i.put_desk(Desk::default());
    assert!(!i.shell(), "跑命令的模式不带到别的会话");
    assert!(!i.stashed(), "暂存不带到别的会话");
    i.editor.insert("早先");
    press(&mut i, KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(i.editor.text(), "早先发过的", "输入历史大家都翻得到");
    let two = i.take_desk();
    i.put_desk(one);
    assert!(i.shell() && i.stashed());
    assert_eq!(i.editor.text(), "ls");
    i.take_desk();
    i.put_desk(two);
    press(&mut i, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(i.editor.text(), "早先", "翻历史之前没发的那句跟着二号");
}
