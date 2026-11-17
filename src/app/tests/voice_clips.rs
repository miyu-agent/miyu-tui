//! 她说出来的语音（核心 SD-1，蓝图「她说的语音」）：这个终端开的那一轮的当场经总线 `tts.play {"audio"}` 放，点语音条停、再点放；
//! 正在看的会话里没人开口的那一轮的也放；别的头开的只画；停放着的会话只放这个终端开的；总线上没有 `tts` 的不放。

use serde_json::{Value, json};

use super::app;
use crate::core::{Command, Push, Spoken, Update, Voice};
use crate::transcript::Kind;

fn clip() -> Voice {
    Voice {
        blob: "sha256:aa".into(),
        media_type: "audio/wav".into(),
        name: Some("say-1.wav".into()),
        duration_ms: Some(1200),
    }
}

/// 一轮：开它的那句话（`asked`：到点叫醒的是真），开它的请求是不是这个终端发的，她说了一段。
fn turn(asked: Option<bool>, mine: bool) -> Vec<Push> {
    let mut pushes: Vec<Push> = asked
        .map(|nobody| Push::Voice(Spoken::Asked { seq: 4, nobody }))
        .into_iter()
        .collect();
    pushes.extend([
        Push::TurnStarted(5, Some(4)),
        Push::Voice(Spoken::Opened {
            trigger: Some(4),
            mine,
        }),
        Push::Voice(Spoken::Clip(clip())),
    ]);
    pushes
}

/// 发出去的 `bus.call`：方法和参数。
fn bus(sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>) -> Vec<Value> {
    std::iter::from_fn(|| sent.try_recv().ok())
        .filter_map(|command| match command {
            Command::Ask {
                method: "bus.call",
                params,
                ..
            } => Some(json!([params["method"], params["params"]])),
            _ => None,
        })
        .collect()
}

#[test]
fn a_voice_this_terminal_asked_for_plays_at_once_and_a_click_stops_and_replays_it() {
    let (mut app, mut sent) = app();
    app.talk.transcript.session = Some("s-1".into());
    app.link.voice.ready = vec!["tts".into()];
    for push in turn(Some(false), true) {
        app.core(Update::Push(push));
    }
    assert_eq!(bus(&mut sent), [json!(["play", {"audio": "sha256:aa"}])]);
    assert_eq!(app.link.voice.clip.as_deref(), Some("sha256:aa"));
    let id = app
        .talk
        .transcript
        .entries
        .iter()
        .find(|e| e.kind == Kind::Voice)
        .map(|e| e.id)
        .expect("画了语音条");
    app.open_link(&format!("miyu-voice:{id}"));
    assert_eq!(bus(&mut sent), [json!(["stop", {}])], "在放的点一下停");
    app.core(Update::Answer {
        tag: crate::oobe::asker::TAG_BASE - 55,
        result: Ok(json!({"finished": false})),
    });
    assert!(app.link.voice.clip.is_none());
    app.open_link(&format!("miyu-voice:{id}"));
    assert_eq!(
        bus(&mut sent),
        [json!(["play", {"audio": "sha256:aa"}])],
        "再点放"
    );
}

#[test]
fn only_this_terminal_or_nobody_opening_the_turn_plays_it() {
    for (asked, mine, plays) in [
        (Some(true), false, true),
        (None, false, true),
        (Some(false), false, false),
    ] {
        let (mut app, mut sent) = app();
        app.talk.transcript.session = Some("s-1".into());
        app.link.voice.ready = vec!["tts".into()];
        for push in turn(asked, mine) {
            app.core(Update::Push(push));
        }
        assert_eq!(!bus(&mut sent).is_empty(), plays, "{asked:?} {mine}");
    }
    // 总线上没有 `tts`：不放，语音条照画。
    let (mut app, mut sent) = app();
    app.talk.transcript.session = Some("s-1".into());
    for push in turn(Some(false), true) {
        app.core(Update::Push(push));
    }
    assert!(bus(&mut sent).is_empty());
    assert!(
        app.talk
            .transcript
            .entries
            .iter()
            .any(|e| e.kind == Kind::Voice)
    );
}

#[test]
fn a_parked_session_plays_only_what_this_terminal_asked_for() {
    for (mine, plays) in [(true, true), (false, false)] {
        let (mut app, mut sent) = app();
        app.talk.transcript.session = Some("s-1".into());
        app.link.voice.ready = vec!["tts".into()];
        app.talk
            .parked
            .insert("s-2".into(), crate::app::sessions::Parked::default());
        for push in turn(None, mine) {
            app.core(Update::Elsewhere {
                session: "s-2".into(),
                update: Box::new(Update::Push(push)),
            });
        }
        assert_eq!(!bus(&mut sent).is_empty(), plays, "{mine}");
    }
}
