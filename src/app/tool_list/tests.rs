//! `/tool-list` 问核心的那一条、回来的放哪（蓝图「工具 `/tool-list`」）。

use serde_json::json;

use super::super::Panel;
use super::super::test_support::app;
use super::TOOLS_TAG;
use crate::core::{Command, Update};

/// 发出去的最后一条 `session.tools` 的参数。
fn asked(sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>) -> serde_json::Value {
    let mut last = None;
    while let Ok(command) = sent.try_recv() {
        if let Command::Ask {
            method: "session.tools",
            params,
            ..
        } = command
        {
            last = Some(params);
        }
    }
    last.expect("问了 session.tools")
}

#[test]
fn an_open_session_is_asked_by_id_and_a_new_one_by_its_picks() {
    let (mut app, mut sent) = app();
    app.open_tool_list();
    assert_eq!(asked(&mut sent), json!({}), "还没选的不写：核心照默认的");
    app.talk.persona.chosen = Some(String::new());
    app.talk.preset.chosen = Some("dev".into());
    app.open_tool_list();
    assert_eq!(
        asked(&mut sent),
        json!({"persona": null, "preset": "dev"}),
        "无人格写 null"
    );
    app.talk.transcript.session = Some("s1".into());
    app.open_tool_list();
    assert_eq!(asked(&mut sent), json!({"session": "s1"}));
}

#[test]
fn the_answer_is_kept_only_while_the_box_is_open() {
    let (mut app, _sent) = app();
    let answer = || Update::Answer {
        tag: TOOLS_TAG,
        result: Ok(
            json!({"tools": [{"name": "read", "source": {"kind": "builtin", "id": "basesystem"}}]}),
        ),
    };
    app.open_tool_list();
    assert!(app.tool_list_update(&answer()));
    assert_eq!(
        app.boxes.tools.as_ref().map(|r| r.as_ref().map(Vec::len)),
        Some(Ok(1))
    );
    app.boxes.panel = None;
    app.boxes.tools = None;
    assert!(app.tool_list_update(&answer()), "还是归它");
    assert!(app.boxes.tools.is_none(), "框关了不要");
    assert!(!matches!(app.boxes.panel, Some(Panel::Tools { .. })));
}
