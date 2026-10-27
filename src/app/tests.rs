//! 界面这一层的行为（不经伪终端）。

use super::new_session::Kind;
use super::test_support::app;
use crate::core::Persona;
use crate::transcript::Link;

fn persona(id: &str, name: &str) -> Persona {
    Persona {
        id: id.into(),
        name: Some(name.into()),
        summary: None,
        problem: None,
        avatar: None,
        seed: None,
        seeds: None,
    }
}

#[test]
fn the_persona_box_opens_from_the_remembered_list_once_connected() {
    // 2026-10-22 项目主人：进界面到人格框出来慢（核心冷启动时列人格要近一秒）。起来时照记下的列表摆好（`look.rs`），
    // 连上、读到配置就开框，不等核心的列表；核心的到了照它换。
    let (mut app, _sent) = app();
    app.listed(Kind::Persona, vec![persona("pirate", "海盗")]);
    assert!(app.boxes.panel.is_none(), "还不知道走没走过引导：不开");
    app.talk.transcript.link = Link::Ready;
    app.picks_configured(None, None);
    let Some(super::Panel::Pick { kind, selected }) = app.boxes.panel else {
        panic!("连上就照记下的开框：{:?}", app.boxes.panel);
    };
    assert_eq!(kind, Kind::Persona);
    let names: Vec<&str> = app
        .talk
        .persona
        .list
        .iter()
        .flatten()
        .map(Persona::label)
        .collect();
    assert_eq!(names, ["无人格", "海盗"], "前面照样加「无人格」");
    assert_eq!(selected, 0);
    // 核心的到了，短了一截、人没挪过：照上一次选的重新停，不越界。
    app.listed(Kind::Persona, vec![]);
    app.listed(Kind::Persona, vec![persona("captain", "船长")]);
    let Some(super::Panel::Pick { selected, .. }) = app.boxes.panel else {
        panic!("框还开着");
    };
    let names: Vec<&str> = app
        .talk
        .persona
        .list
        .iter()
        .flatten()
        .map(Persona::label)
        .collect();
    assert_eq!(names, ["无人格", "船长"]);
    assert!(selected < names.len());
}

#[test]
fn the_persona_box_opens_before_connecting_when_the_guide_was_done() {
    // 同一天项目主人：「依旧不是秒开」，照推荐定连上以前就照记下的开（记着走过引导的才开，不和引导叠在一起）。
    let (mut app, mut sent) = app();
    assert_ne!(app.talk.transcript.link, Link::Ready);
    app.boxes.welcome_known = true;
    app.listed(Kind::Persona, vec![persona("pirate", "海盗")]);
    assert!(
        matches!(
            app.boxes.panel,
            Some(super::Panel::Pick {
                kind: Kind::Persona,
                ..
            })
        ),
        "没连上也开：{:?}",
        app.boxes.panel
    );
    // 挪到海盗按 Enter：选的排进命令里，连上再发。
    let enter = |code| {
        ratatui::crossterm::event::KeyEvent::new(
            code,
            ratatui::crossterm::event::KeyModifiers::NONE,
        )
    };
    app.pick_key(
        Kind::Persona,
        1,
        enter(ratatui::crossterm::event::KeyCode::Enter),
    );
    assert!(matches!(sent.try_recv(), Ok(crate::core::Command::Persona(id)) if id == "pirate"));
}

mod bill;
mod goal;
mod pools;
mod status;
