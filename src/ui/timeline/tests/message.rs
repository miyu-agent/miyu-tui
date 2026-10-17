//! 点开一条留言（`send_message`，2026-10-18 项目主人：原来只有「Message sent to 35106d22.」）：发给谁（会话的标题、子代理的
//! 名字、父会话）、完整的消息、附件。

use std::time::Instant;

use serde_json::{Value, json};

use super::{rows, segment, step, text};
use crate::core::{JobStart, ToolStatus};
use crate::transcript::{Step, StepKind, ToolState};
use crate::ui::test_support::Fixture;

const SESSION: &str = "019a5c3e-8f2b-7c61-9e0d-35106d22aaaa";

fn message(parsed: Value) -> Step {
    let mut s = step(
        StepKind::Tool {
            name: "send_message".into(),
            args: String::new(),
            parsed,
            state: ToolState::Done(ToolStatus::Ok),
            output: "Message sent to 35106d22.".into(),
            said: None,
        },
        Instant::now(),
        0,
        1,
    );
    s.open = Some(true);
    s
}

fn opened(f: &Fixture, step: Step) -> Vec<String> {
    text(&rows(0, &segment(vec![step], Some(true)), &f.ctx()))
}

#[test]
fn a_message_to_another_session_shows_its_title_and_the_whole_message() {
    let mut f = Fixture::new();
    f.sessions = vec![crate::core::SessionInfo {
        session: SESSION.into(),
        title: Some("niri 会话".into()),
        ..Default::default()
    }];
    let got = opened(
        &f,
        message(
            json!({"to": SESSION, "message": "第一行\n第二行", "attachments": ["/tmp/a/shot.png"]}),
        ),
    );
    assert!(
        got.iter().any(|l| l.contains("→ niri 会话 · 会话 ")),
        "{got:?}"
    );
    assert!(got.iter().any(|l| l.contains("第一行")), "{got:?}");
    assert!(
        got.iter().any(|l| l.contains("第二行")),
        "完整的消息：{got:?}"
    );
    assert!(got.iter().any(|l| l.contains("附件：shot.png")), "{got:?}");
    assert!(got.iter().all(|l| !l.contains("Message sent")), "{got:?}");
}

#[test]
fn a_message_to_a_subagent_or_the_parent_names_it() {
    let mut f = Fixture::new();
    f.board.start(
        &JobStart {
            call_id: "c".into(),
            job: "j3".into(),
            agent: true,
            title: "查文档".into(),
            session: Some("s-child".into()),
            foreground: false,
        },
        None,
        Instant::now(),
    );
    let got = opened(&f, message(json!({"to": "j3", "message": "再看一遍"})));
    assert!(
        got.iter().any(|l| l.contains("→ 子代理 查文档 · j3")),
        "{got:?}"
    );
    let got = opened(&f, message(json!({"to": "parent", "message": "好了"})));
    assert!(got.iter().any(|l| l.contains("→ 父会话")), "{got:?}");
    // 不认识的会话：只写短编号。
    let got = opened(&f, message(json!({"to": SESSION, "message": "在吗"})));
    assert!(got.iter().any(|l| l.contains("→ 会话 ")), "{got:?}");
}
