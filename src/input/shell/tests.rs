use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::input::{Action, Draft, InputBox};

/// 一个空的输入框，翻历史要它知道多宽（`vertical` 照宽度折行）。
fn input_box() -> InputBox {
    let mut i = InputBox::new(8, std::time::Duration::from_millis(400));
    i.place(ratatui::layout::Rect::new(0, 0, 40, 3));
    i
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn a_bang_in_an_empty_box_enters_the_mode_and_backspace_leaves_it() {
    // 「人自己跑命令 `!`」第 1 条：`!` 不写进框，空着按 Backspace、Esc 回到平常。
    let mut input = input_box();
    input.key(key(KeyCode::Char('!')));
    assert!(input.shell());
    assert_eq!(input.editor.text(), "");
    input.key(key(KeyCode::Char('l')));
    input.key(key(KeyCode::Char('s')));
    assert_eq!(input.editor.text(), "ls");
    input.key(key(KeyCode::Backspace));
    input.key(key(KeyCode::Backspace));
    assert!(input.shell(), "有字时 Backspace 照常删字");
    input.key(key(KeyCode::Backspace));
    assert!(!input.shell(), "空着按 Backspace 回到平常");
    input.key(key(KeyCode::Char('!')));
    input.key(key(KeyCode::Esc));
    assert!(!input.shell(), "空着按 Esc 也回到平常");
}

#[test]
fn pasted_and_recalled_commands_come_back_in_the_mode() {
    // 第 1、5 条：粘进来的、`↑` 翻出来的以 `!` 打头的照样进这个模式，输入历史里记成 `!` 打头的一条。
    let mut input = input_box();
    input.paste("!git status");
    assert!(input.shell());
    assert_eq!(input.editor.text(), "git status");
    let Action::Submit(draft) = input.key(key(KeyCode::Enter)) else {
        panic!("回车交出去");
    };
    let typed = input.as_typed(&draft);
    assert_eq!(typed.text, "!git status");
    input.leave_shell();
    input.remember(typed);
    input.remember(Draft::plain("你好"));
    input.key(key(KeyCode::Up));
    assert!(!input.shell(), "翻到普通的话");
    input.key(key(KeyCode::Up));
    assert!(input.shell(), "翻到跑命令的那一条");
    assert_eq!(input.editor.text(), "git status");
    assert!(input.recalled(), "还算翻出来的、没改过：接着翻");
    input.key(key(KeyCode::Down));
    assert!(!input.shell());
    assert_eq!(input.editor.text(), "你好");
}

#[test]
fn a_command_picked_from_the_history_list_comes_back_in_the_mode() {
    // 第 5 条：`Ctrl+R` 挑出来的 `!` 打头的照样进跑命令；挑普通的回到平常。
    let mut input = input_box();
    input.remember(Draft::plain("!cargo test"));
    input.pick("!cargo test");
    assert!(input.shell());
    assert_eq!(input.editor.text(), "cargo test");
    input.editor.take();
    input.pick("你好");
    assert!(!input.shell());
}
