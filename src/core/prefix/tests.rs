use serde_json::json;

use super::{Opened, changing};
use crate::core::awaiting::Awaiting;
use crate::core::link::Link;
use crate::core::update::Update;

fn refused(data: serde_json::Value) -> serde_json::Value {
    json!({"code": -32000, "message": "prefix changing", "data": data})
}

#[test]
fn a_prefix_change_on_a_turn_is_asked_with_its_files() {
    let link = Link::default();
    let error = refused(json!({"reason": "prefix_changing",
        "files": ["root/personas/alpha/prompts/persona.md"], "more": 2}));
    let Some(Update::PrefixChanging {
        files,
        more,
        command,
    }) = changing(&error, Some(&Awaiting::Send), &link)
    else {
        panic!("开轮的要问");
    };
    assert_eq!(files, ["root/personas/alpha/prompts/persona.md"]);
    assert_eq!((more, command), (2, None));
    assert!(
        changing(&error, Some(&Awaiting::Recap), &link).is_none(),
        "不开轮的不问"
    );
    let other = refused(json!({"reason": "turn_running"}));
    assert!(changing(&other, Some(&Awaiting::Send), &link).is_none());
}

#[test]
fn a_turn_opening_command_hands_back_its_text_for_cancel() {
    let link = Link {
        opened: Some(Opened {
            method: "command.run",
            params: json!({"session": "s", "text": "/review"}),
            kind: Awaiting::Slash,
        }),
        ..Link::default()
    };
    let error = refused(json!({"reason": "prefix_changing", "files": []}));
    let Some(Update::PrefixChanging { command, more, .. }) =
        changing(&error, Some(&Awaiting::Slash), &link)
    else {
        panic!("要问");
    };
    assert_eq!((command.as_deref(), more), (Some("/review"), 0));
}

#[test]
fn a_plain_message_hands_back_no_command() {
    // 说的话也带 `text`：取消照「没发出去」放回框里，不当成命令。
    let link = Link {
        opened: Some(Opened {
            method: "session.send",
            params: json!({"session": "s", "text": "你好"}),
            kind: Awaiting::Send,
        }),
        ..Link::default()
    };
    let error = refused(json!({"reason": "prefix_changing", "files": []}));
    let Some(Update::PrefixChanging { command, .. }) =
        changing(&error, Some(&Awaiting::Send), &link)
    else {
        panic!("要问");
    };
    assert_eq!(command, None);
}
