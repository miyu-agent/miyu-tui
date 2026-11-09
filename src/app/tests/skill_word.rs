//! `$` 点名技能（蓝图「`$` 点名技能」，发版前 check 26，2026-11-09 项目主人）：列表里选定把词换成「`$名字 `」，发出去时点了的
//! 技能另交 `session.send` 的 `skills`、字照原样；Vim 模式下在列表里选时输入法切英文。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::core::{Command, CoreSkill};
use crate::input::Draft;

fn skill(name: &str) -> CoreSkill {
    CoreSkill {
        name: name.into(),
        summary: String::new(),
    }
}

fn press(app: &mut crate::app::App, code: KeyCode) {
    app.key(KeyEvent::new(code, KeyModifiers::NONE));
}

#[test]
fn picking_a_skill_writes_it_into_the_box_and_sending_names_it() {
    let (mut app, mut sent) = crate::app::test_support::app();
    // 开着的会话（不画新会话开头的人格、预设框）。
    app.talk.transcript.session = Some("s1".into());
    app.config.commands.skills = vec![skill("pdf"), skill("review")];
    for c in "看 $rev".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    assert!(app.skill_found().is_some(), "$ 词开列表");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.typing.input.editor.text(), "看 $review ");
    assert!(app.skill_found().is_none(), "选定以后关上");
    while sent.try_recv().is_ok() {}
    app.submit(Draft::plain("看 $review 和 $pdf，再 $review"));
    let mut got = None;
    while let Ok(command) = sent.try_recv() {
        if let Command::Send { text, skills, .. } = command {
            got = Some((text, skills));
        }
    }
    let (text, skills) = got.expect("发了");
    assert_eq!(text, "看 $review 和 $pdf，再 $review", "字照原样");
    assert_eq!(skills, ["review", "pdf"], "点了的照先后、不重复");
}

#[test]
fn without_skills_from_the_core_a_dollar_opens_nothing() {
    // 旧核心（SK-1 以前）不给顶层的 `skills`：技能还在命令里，`$` 照常是字。
    let (mut app, _sent) = crate::app::test_support::app();
    app.talk.transcript.session = Some("s1".into());
    for c in "$5".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    assert!(app.skill_found().is_none());
}

#[test]
fn picking_in_vim_mode_turns_the_input_method_off() {
    let (mut app, _sent) = crate::app::test_support::app();
    app.talk.transcript.session = Some("s1".into());
    app.config.layout.vim = true;
    app.config.commands.skills = vec![skill("pdf")];
    press(&mut app, KeyCode::Char('$'));
    press(&mut app, KeyCode::Esc);
    app.tick();
    assert_eq!(app.typing.ime.wants_typing(), Some(false), "在列表里选");
    press(&mut app, KeyCode::Char('j'));
    assert_eq!(app.typing.input.editor.text(), "$", "j 不进输入框");
}
