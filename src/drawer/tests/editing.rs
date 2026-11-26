//! 写字的地方和主输入框同一套编辑键（2026-11-27 项目主人：光标不能移动，中间打错只能删回去）：光标挪到中间插入、删除，
//! 多行里上下走，`Home` `End`、按词跳、按词删；`Esc` 的留和清、三处各自的保存照旧。

use super::*;

fn with(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

fn newline(d: &mut Drawer) {
    d.key(with(KeyCode::Enter, KeyModifiers::SHIFT));
}

/// 单选题，在「其他」上进了编辑。
fn writing_other() -> Drawer {
    let mut d = Drawer::question(None, asked());
    assert_eq!(d.key(key(KeyCode::Char('3'))), Step::Stay);
    assert_eq!(d.editing, Some(Edit::Other));
    d
}

#[test]
fn the_cursor_moves_and_edits_happen_where_it_is() {
    let mut d = writing_other();
    typed(&mut d, "abcd");
    d.key(key(KeyCode::Left));
    d.key(key(KeyCode::Left));
    typed(&mut d, "X");
    assert_eq!(d.typed[0], "abXcd", "插在光标处，不是接在末尾");
    d.key(key(KeyCode::Delete));
    assert_eq!(d.typed[0], "abXd", "删除键删光标后面那个字");
    d.key(key(KeyCode::Backspace));
    assert_eq!(d.typed[0], "abd", "退格删光标前面那个字");
    d.key(key(KeyCode::Home));
    typed(&mut d, "^");
    d.key(key(KeyCode::End));
    typed(&mut d, "$");
    assert_eq!(d.typed[0], "^abd$", "Home、End 到这一行的两头");
    // 中文、emoji 一个字一下（同输入框按字素簇走）。
    let mut d = writing_other();
    typed(&mut d, "你👍🏽好");
    d.key(key(KeyCode::Left));
    d.key(key(KeyCode::Left));
    typed(&mut d, "们");
    assert_eq!(d.typed[0], "你们👍🏽好");
}

#[test]
fn up_and_down_walk_the_lines_and_stop_at_the_ends() {
    let mut d = writing_other();
    typed(&mut d, "第一行");
    newline(&mut d);
    typed(&mut d, "第二行");
    d.key(key(KeyCode::Up));
    typed(&mut d, "改");
    assert_eq!(d.typed[0], "第一行改\n第二行", "↑ 回到第一行的同一列");
    // 第一行再往上：到开头，不退出编辑、不换选项。
    d.key(key(KeyCode::Up));
    typed(&mut d, "<");
    assert_eq!(d.editing, Some(Edit::Other));
    assert_eq!(d.cursor[0], 2, "还在「其他」上");
    d.key(key(KeyCode::Down));
    d.key(key(KeyCode::Down));
    typed(&mut d, ">");
    assert_eq!(d.typed[0], "<第一行改\n第二行>", "最后一行再往下：到结尾");
}

#[test]
fn word_keys_jump_and_delete_like_the_input_box() {
    let mut d = writing_other();
    typed(&mut d, "hello brave world");
    d.key(with(KeyCode::Left, KeyModifiers::CONTROL));
    d.key(with(KeyCode::Char('w'), KeyModifiers::CONTROL));
    assert_eq!(
        d.typed[0], "hello world",
        "Ctrl+← 跳到词头，Ctrl+W 删前一个词"
    );
    d.key(with(KeyCode::Char('a'), KeyModifiers::CONTROL));
    typed(&mut d, ">");
    d.key(with(KeyCode::Right, KeyModifiers::CONTROL));
    d.key(with(KeyCode::Backspace, KeyModifiers::ALT));
    assert_eq!(
        d.typed[0], "> world",
        "Ctrl+A 到开头，Ctrl+→ 跳到词尾，Alt+Backspace 删一个词"
    );
    d.key(with(KeyCode::Char('j'), KeyModifiers::CONTROL));
    d.key(with(KeyCode::Enter, KeyModifiers::ALT));
    assert_eq!(d.typed[0], ">\n\n world", "Ctrl+J、Alt+Enter 在光标处换行");
}

#[test]
fn esc_keeps_other_where_it_was_and_clears_notes_and_reasons() {
    let mut d = writing_other();
    typed(&mut d, "先这样");
    d.key(key(KeyCode::Left));
    d.key(key(KeyCode::Esc));
    assert_eq!(d.editing, None);
    assert_eq!(d.typed[0], "先这样", "「其他」的字留着");
    // 再进编辑：光标还在原来的地方。
    d.key(key(KeyCode::Enter));
    typed(&mut d, "说");
    assert_eq!(d.typed[0], "先这说样");
    // 补充：Esc 清空，再按 n 是空的。
    let mut d = Drawer::question(None, asked());
    d.key(key(KeyCode::Char('n')));
    typed(&mut d, "顺便");
    d.key(key(KeyCode::Left));
    d.key(key(KeyCode::Esc));
    assert_eq!(d.notes[0], "");
    d.key(key(KeyCode::Char('n')));
    typed(&mut d, "再写");
    assert_eq!(d.notes[0], "再写");
    // 理由：同补充。
    let mut d = Drawer::approval(None, approval());
    d.key(key(KeyCode::Char('n')));
    typed(&mut d, "不行");
    d.key(key(KeyCode::Home));
    d.key(key(KeyCode::Esc));
    assert_eq!(d.typed[0], "");
}

#[test]
fn enter_saves_each_of_the_three_with_its_lines() {
    // 「其他」单选：答了这一道，换行照原样交出去，光标挪过、改过的照改好的。
    let mut d = writing_other();
    typed(&mut d, "第一行");
    newline(&mut d);
    typed(&mut d, "第二行");
    d.key(key(KeyCode::Up));
    d.key(key(KeyCode::Left));
    d.key(key(KeyCode::Backspace));
    typed(&mut d, "二");
    let got = answers(d.key(key(KeyCode::Enter)));
    assert_eq!(got[0].text.as_deref(), Some("第二行\n第二行"));
    // 「其他」多选：算勾上，不往下走。
    let mut d = Drawer::question(None, two());
    d.key(key(KeyCode::Right));
    d.key(key(KeyCode::Char('4')));
    typed(&mut d, "丁");
    newline(&mut d);
    typed(&mut d, "戊");
    assert_eq!(d.key(key(KeyCode::Enter)), Step::Stay);
    assert_eq!(
        d.answers[1].as_ref().and_then(|a| a.text.as_deref()),
        Some("丁\n戊")
    );
    // 补充：只是存下，交的时候接进这一道。
    let mut d = Drawer::question(None, asked());
    d.key(key(KeyCode::Char('n')));
    typed(&mut d, "快");
    d.key(key(KeyCode::Home));
    typed(&mut d, "要");
    assert_eq!(d.key(key(KeyCode::Enter)), Step::Stay);
    let got = answers(d.key(key(KeyCode::Char('1'))));
    assert_eq!(got[0].notes.as_deref(), Some("要快"));
    // 理由：交「不允许」。
    let mut d = Drawer::approval(None, approval());
    d.key(key(KeyCode::Char('n')));
    typed(&mut d, "别动");
    newline(&mut d);
    typed(&mut d, "家目录");
    d.key(key(KeyCode::Up));
    d.key(key(KeyCode::End));
    typed(&mut d, "我的");
    let Step::Done(Outcome::Decided(x)) = d.key(key(KeyCode::Enter)) else {
        panic!("没交");
    };
    assert_eq!(
        (x.decision, x.reason.as_deref()),
        (Decision::Deny, Some("别动我的\n家目录"))
    );
}
