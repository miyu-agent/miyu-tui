//! 别处来的话的来处（蓝图 `tui.md`「别处来的话」第 2 条）：同一个人在别的头上说的写是哪个头（核心 VN-1 的 `by.head`）；看场所
//! 会话时，平台上的人写「昵称（qq:号）」、管理员接「 · 管理员」，戳了一下的画一行暗字（核心 VW-2、VP-1补，同网页 W-1210、W-1212）。

use serde_json::{Value, json};

use super::app;
use crate::core::{Update, read_live};
use crate::transcript::{Entry, Kind};

fn said(seq: u64, by: Value, text: &str, venue: Value) -> Value {
    json!({"seq": seq, "at": "2026-11-18T08:00:00Z", "kind": "message.user", "by": by, "cause": format!("x-{seq}"),
           "body": {"blocks": [{"type": "text", "text": text}], "venue": venue}})
}

fn qq(id: &str, role: &str) -> Value {
    json!({"kind": "external", "venue": "qq:group:960101", "id": id, "role": role})
}

/// 读进正在看的会话 `s-1` 以后正文里的这几条：来处（没有的是种类）和字。
fn drawn(events: &[Value]) -> Vec<(String, String)> {
    let (mut app, _sent) = app();
    app.talk.transcript.session = Some("s-1".into());
    for push in read_live(events) {
        app.core(Update::Push(push));
    }
    let row = |e: &Entry| {
        (
            e.from.clone().unwrap_or(format!("{:?}", e.kind)),
            e.text.clone(),
        )
    };
    app.talk.transcript.entries.iter().map(row).collect()
}

#[test]
fn what_you_said_on_another_head_says_which_head() {
    let got = drawn(&[
        said(
            2,
            json!({"kind": "person", "head": "web"}),
            "网页上说的",
            json!(null),
        ),
        said(
            3,
            json!({"kind": "person", "head": "shellhook"}),
            "shell 里说的",
            json!(null),
        ),
        said(4, json!({"kind": "person"}), "老头说的", json!(null)),
    ]);
    assert_eq!(
        got,
        [
            ("收到你在网页发送的消息".into(), "网页上说的".into()),
            ("收到你在终端集成发送的消息".into(), "shell 里说的".into()),
            ("收到你在其他端发送的消息".into(), "老头说的".into()),
        ]
    );
}

#[test]
fn venue_speakers_show_nickname_and_number_and_a_poke_is_one_dim_line() {
    let got = drawn(&[
        said(
            2,
            qq("qq:20001", "member"),
            "今天有人在吗",
            json!({"msg": "1", "name": "阿杰"}),
        ),
        said(
            3,
            qq("qq:20002", "manager"),
            "在",
            json!({"msg": "2", "name": "小美"}),
        ),
        said(
            4,
            qq("qq:20001", "member"),
            "",
            json!({"msg": "3", "name": "阿杰", "poke": true, "poke_text": "的小脑袋"}),
        ),
        said(
            5,
            qq("qq:20002", "manager"),
            "",
            json!({"msg": "4", "name": "小美", "poke": true}),
        ),
    ]);
    assert_eq!(
        got,
        [
            ("阿杰（qq:20001）".into(), "今天有人在吗".into()),
            ("小美（qq:20002） · 管理员".into(), "在".into()),
            ("Note".into(), "阿杰（qq:20001） 戳了戳你的小脑袋".into()),
            ("Note".into(), "小美（qq:20002） 戳了戳你".into()),
        ]
    );
}

#[test]
fn a_turn_opened_by_a_poke_takes_the_poke_line() {
    let (mut app, _sent) = app();
    app.talk.transcript.session = Some("s-1".into());
    let events = [
        said(
            4,
            qq("qq:20001", "member"),
            "",
            json!({"msg": "3", "name": "阿杰", "poke": true}),
        ),
        json!({"seq": 5, "kind": "turn.started", "turn": 5, "body": {"trigger": 4}}),
    ];
    for push in read_live(&events) {
        app.core(Update::Push(push));
    }
    let poke = app
        .talk
        .transcript
        .entries
        .iter()
        .find(|e| e.kind == Kind::Note);
    assert_eq!(
        poke.and_then(|e| e.turn),
        Some(5),
        "戳的那一行归到它开的那一轮"
    );
}
