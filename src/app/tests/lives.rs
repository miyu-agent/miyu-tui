//! 快速跳转人生会话（「推演人生」第 1 条，2026-11-17 项目主人本机验收第 29 条，核心 LF-12）：光打 `/life` 带回的人生一条的
//! 跳过去、几条的开列表挑；还没开会话时不开会话，直接 `life.list`。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::json;

use super::app;
use crate::app::Panel;
use crate::core::{Command, Life, Update};

fn life(session: &str, name: &str) -> Life {
    Life {
        session: session.into(),
        persona: name.into(),
        name: name.into(),
        ..Life::default()
    }
}

fn ran(lives: Vec<Life>) -> Update {
    Update::CommandRan {
        command: "life".into(),
        said: "人生会话进行中，下次继续：14:05。".into(),
        session: lives.first().map(|l| l.session.clone()),
        lives,
    }
}

/// 发出去的：打开了哪个会话、问了哪个方法。
fn sent_of(sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>) -> Vec<String> {
    let mut got = Vec::new();
    while let Ok(command) = sent.try_recv() {
        match command {
            Command::Open { session, .. } => got.push(format!("open {session}")),
            Command::Ask { method, .. } => got.push(method.to_string()),
            Command::Slash(line) => got.push(line),
            _ => {}
        }
    }
    got
}

#[test]
fn one_life_jumps_and_several_open_a_list_to_pick_from() {
    let (mut app, mut sent) = app();
    app.talk.transcript.session = Some("s-here".into());
    let kept = app.follow_ran(ran(vec![life("s-a", "美羽")]));
    assert_eq!(sent_of(&mut sent), ["open s-a"], "一条直接跳");
    assert!(
        matches!(kept, Update::CommandRan { ref said, .. } if !said.is_empty()),
        "回执照常弹"
    );
    let kept = app.follow_ran(ran(vec![life("s-a", "美羽"), life("s-b", "露娜")]));
    assert!(sent_of(&mut sent).is_empty(), "几条的不跳");
    assert!(matches!(
        app.boxes.panel,
        Some(Panel::Lives { selected: 0 })
    ));
    assert!(
        matches!(kept, Update::CommandRan { ref said, .. } if said.is_empty()),
        "开了列表的不再弹回执"
    );
    app.lives_key(0, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert!(matches!(
        app.boxes.panel,
        Some(Panel::Lives { selected: 1 })
    ));
    app.lives_key(1, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(sent_of(&mut sent), ["open s-b"]);
    assert!(app.boxes.panel.is_none());
}

#[test]
fn a_session_not_yet_started_asks_the_list_without_opening_one() {
    // 2026-11-17 核心 LF-12 实测：网页空会话打 /life 只说「Start a session first」；终端原来先开一个会话再交给核心。
    let (mut app, mut sent) = app();
    assert!(app.talk.transcript.session.is_none());
    app.slash("/life");
    assert_eq!(
        sent_of(&mut sent),
        ["life.list"],
        "不开会话、不交 command.run"
    );
    app.core(Update::Answer {
        tag: crate::oobe::asker::TAG_BASE - 51,
        result: Ok(json!({"lives": [
            {"state": "running", "session": "s-a", "persona": "miyu", "name": "美羽"},
            {"state": "paused", "session": "s-b", "persona": "luna", "name": "露娜"}
        ], "ended_unseen": []})),
    });
    assert!(matches!(app.boxes.panel, Some(Panel::Lives { .. })));
    app.boxes.panel = None;
    app.core(Update::Answer {
        tag: crate::oobe::asker::TAG_BASE - 51,
        result: Ok(json!({"lives": [], "ended_unseen": []})),
    });
    assert_eq!(
        app.typing.notice.as_ref().map(|n| n.text.as_str()),
        Some("没有正在过的人生")
    );
    // 别的 `/life …` 照旧交给核心。
    app.slash("/life pause");
    assert_eq!(sent_of(&mut sent), ["/life pause"]);
}

#[test]
fn life_new_in_a_session_not_yet_started_starts_the_life_without_an_empty_session() {
    // 2026-11-17 本机验收的剩项：原来先开一个空会话、再交 `/life new`，留下一条「未命名会话」。
    let (mut app, mut sent) = app();
    app.talk.persona.chosen = Some("miyu".into());
    app.slash("/life  new ");
    let mut asked = Vec::new();
    while let Ok(command) = sent.try_recv() {
        match command {
            Command::Ask { method, params, .. } => asked.push((method, params)),
            other => panic!("只问 life.new，不开会话、不交命令：{other:?}"),
        }
    }
    assert_eq!(asked, [("life.new", json!({"persona": "miyu"}))]);
    app.core(Update::Answer {
        tag: crate::oobe::asker::TAG_BASE - 54,
        result: Ok(json!({"session": "s-life"})),
    });
    assert_eq!(
        sent_of(&mut sent),
        ["open s-life"],
        "开出来的人生会话跳过去"
    );
}

#[test]
fn life_new_without_a_persona_says_to_pick_one_first() {
    // 选的是无人格：开不了，说一句，什么都不发。
    let (mut app, mut sent) = app();
    app.talk.persona.chosen = Some(String::new());
    app.slash("/life new");
    assert!(sent_of(&mut sent).is_empty());
    assert_eq!(
        app.typing.notice.as_ref().map(|n| n.text.as_str()),
        Some("先选一个人格，再开始人生会话")
    );
}
