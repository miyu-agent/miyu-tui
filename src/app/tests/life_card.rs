//! 人生卡（「推演人生」第 7 条，2026-11-17 项目主人本机验收第 23 条）：人生会话里点人生记号开卡，暂停、继续、结束照
//! `life.*` 调（通用的定时方法回 `life_schedule`），结束要按两下。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Position, Rect};

use super::app;
use crate::app::Panel;
use crate::core::{Command, Loop, LoopEvery, LoopLife};

fn life(paused: bool) -> Loop {
    Loop {
        id: "t9".into(),
        session: "s-life".into(),
        every: LoopEvery::Life,
        next: Some("2026-11-17T09:00:00Z".parse().unwrap()),
        paused,
        prompt: "推演方法".into(),
        life: Some(LoopLife {
            persona: Some("miyu".into()),
            ..LoopLife::default()
        }),
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// 发出去的 `life.*`：方法和人格。
fn asked(sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>) -> Vec<(String, String)> {
    let mut got = Vec::new();
    while let Ok(command) = sent.try_recv() {
        if let Command::Ask { method, params, .. } = command
            && method.starts_with("life.")
        {
            got.push((
                method.to_string(),
                params["persona"].as_str().unwrap_or_default().into(),
            ));
        }
    }
    got
}

#[test]
fn the_life_mark_opens_the_card_whose_buttons_call_life_methods() {
    let (mut app, mut sent) = app();
    app.talk.transcript.session = Some("s-life".into());
    app.link.loops = vec![life(false)];
    app.screen.areas.loop_text = Rect::new(70, 20, 8, 1);
    app.screen.pointer = Some(Position::new(72, 20));
    assert!(app.pointing(), "悬停在人生记号上：手");
    assert!(app.life_clicked(Position::new(72, 20)), "点了开卡");
    assert!(matches!(
        app.boxes.panel,
        Some(Panel::Life {
            button: 0,
            arming: false
        })
    ));
    // 停在「暂停」上按 Enter：life.pause，卡还开着。
    app.life_card_key((0, false), key(KeyCode::Enter));
    assert_eq!(asked(&mut sent), [("life.pause".into(), "miyu".into())]);
    assert!(app.boxes.panel.is_some());
    // 暂停着的第一个按钮是「继续」。
    app.link.loops = vec![life(true)];
    app.life_card_key((0, false), key(KeyCode::Enter));
    assert_eq!(asked(&mut sent), [("life.resume".into(), "miyu".into())]);
    // 结束：按一下只变「确认结束」，不发；再按才 life.kill，卡关上。
    app.life_card_key((0, false), key(KeyCode::Right));
    app.life_card_key((1, false), key(KeyCode::Enter));
    assert!(matches!(
        app.boxes.panel,
        Some(Panel::Life {
            button: 1,
            arming: true
        })
    ));
    assert!(asked(&mut sent).is_empty(), "按一下不结束");
    app.life_card_key((1, true), key(KeyCode::Enter));
    assert_eq!(asked(&mut sent), [("life.kill".into(), "miyu".into())]);
    assert!(app.boxes.panel.is_none());
}

#[test]
fn an_ordinary_loop_does_not_open_the_life_card() {
    let (mut app, _sent) = app();
    app.talk.transcript.session = Some("s-1".into());
    let mut plain = life(false);
    plain.session = "s-1".into();
    plain.every = LoopEvery::Dynamic;
    plain.life = None;
    app.link.loops = vec![plain];
    app.screen.areas.loop_text = Rect::new(70, 20, 8, 1);
    assert!(!app.life_clicked(Position::new(72, 20)));
    assert!(app.boxes.panel.is_none());
}
