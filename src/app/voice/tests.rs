//! 语音（设计 20 第七节）：总线上有了 `stt`、`tts` 才列命令；录完的字进输入框，没听清的不放；出错照出错码说；朗读念上一轮的回答。

use serde_json::{Value, json};

use super::super::test_support::app;
use crate::core::{Block, Command, EndReason, Push, Refusal, Update};
use crate::voice::Mic;

/// 发出去的 `bus.call`：编号、调的名字和方法、参数。
fn calls(sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>) -> Vec<(u64, Value)> {
    std::iter::from_fn(|| sent.try_recv().ok())
        .filter_map(|command| match command {
            Command::Ask {
                tag,
                method: "bus.call",
                params,
                ..
            } => Some((tag, params)),
            _ => None,
        })
        .collect()
}

/// 她答了一轮 `text`（照推送喂给正文）。
fn replied(app: &mut crate::app::App, text: &str) {
    let texts = app.config.text.clone();
    let pushes = [
        Push::TurnStarted(1, None),
        Push::BlockStart {
            index: 0,
            block: Block::Text,
        },
        Push::Delta {
            index: 0,
            text: text.into(),
        },
        Push::BlockEnd(0),
    ];
    for push in pushes {
        app.talk.transcript.update(Update::Push(push), &texts);
    }
}

fn names(ready: &[&str]) -> Vec<String> {
    ready.iter().map(|n| (*n).to_string()).collect()
}

#[test]
fn the_commands_show_up_only_when_the_bus_has_them() {
    let (mut app, _sent) = app();
    let listed = |app: &crate::app::App| {
        app.config
            .commands
            .filter("")
            .iter()
            .map(|s| s.name.clone())
            .collect::<Vec<_>>()
    };
    assert!(
        !listed(&app).contains(&"voice-input".to_string()),
        "没装语音包"
    );
    app.core(Update::Bus(names(&["stt"])));
    let now = listed(&app);
    assert!(now.contains(&"voice-input".to_string()));
    assert!(!now.contains(&"read-aloud".to_string()), "没有 tts");
    app.core(Update::Bus(names(&["tts"])));
    assert!(
        !listed(&app).contains(&"voice-input".to_string()),
        "stt 不可用了"
    );
    assert!(listed(&app).contains(&"auto-read".to_string()));
}

#[test]
fn dictation_records_stops_and_puts_the_words_in_the_box() {
    let (mut app, mut sent) = app();
    app.core(Update::Bus(names(&["stt"])));
    app.voice_input();
    let got = calls(&mut sent);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].1["name"], "stt");
    assert_eq!(got[0].1["method"], "record");
    assert_eq!(app.link.voice.mic, Mic::Listening);
    let record = got[0].0;
    // 再点一次：停，等字回来。
    app.voice_input();
    let stop = calls(&mut sent);
    assert_eq!(stop[0].1["method"], "stop");
    assert_eq!(app.link.voice.mic, Mic::Recognizing);
    app.core(Update::Answer {
        tag: record,
        result: Ok(json!({"text": "今天三点开会", "clear": true, "ended": "stop"})),
    });
    assert_eq!(app.typing.input.editor.text(), "今天三点开会", "插进输入框");
    assert_eq!(app.link.voice.mic, Mic::Idle);
    assert!(calls(&mut sent).is_empty(), "不自动发");
}

#[test]
fn an_unclear_take_or_a_busy_microphone_only_says_so() {
    let (mut app, mut sent) = app();
    app.core(Update::Bus(names(&["stt"])));
    app.voice_input();
    let record = calls(&mut sent)[0].0;
    app.core(Update::Answer {
        tag: record,
        result: Ok(json!({"text": "", "clear": false})),
    });
    assert!(app.typing.input.editor.is_empty(), "不放空字");
    let unclear = app.config.text.voice.unclear.clone();
    assert_eq!(
        app.typing.notice.as_ref().map(|n| n.text.clone()),
        Some(unclear)
    );
    app.voice_input();
    let record = calls(&mut sent)[0].0;
    app.core(Update::Answer {
        tag: record,
        result: Err(Refusal {
            reason: Some("method_failed".into()),
            message: "mic busy".into(),
            data: json!({"code": "mic_busy"}),
        }),
    });
    let said = app.typing.notice.as_ref().map(|n| n.text.clone());
    assert_eq!(said.as_deref(), Some("麦克风正被占用"));
    assert_eq!(app.link.voice.mic, Mic::Idle);
}

#[test]
fn reading_aloud_plays_the_last_reply_and_a_second_press_stops_it() {
    let (mut app, mut sent) = app();
    app.core(Update::Bus(names(&["tts"])));
    app.read_aloud();
    assert!(calls(&mut sent).is_empty(), "还没有回答");
    assert_eq!(
        app.typing.notice.as_ref().map(|n| n.text.clone()),
        Some(app.config.text.voice.nothing_to_read.clone())
    );
    replied(&mut app, "**好的**，三点见。");
    app.read_aloud();
    let play = calls(&mut sent);
    assert_eq!(play[0].1["name"], "tts");
    assert_eq!(play[0].1["method"], "play");
    assert_eq!(play[0].1["params"]["text"], "**好的**，三点见。");
    assert!(app.link.voice.speaking);
    app.read_aloud();
    assert_eq!(calls(&mut sent)[0].1["method"], "stop", "在念的停");
    app.core(Update::Answer {
        tag: play[0].0,
        result: Ok(json!({"finished": false})),
    });
    assert!(!app.link.voice.speaking);
}

#[test]
fn auto_read_speaks_each_finished_turn_only_while_it_is_on() {
    let (mut app, mut sent) = app();
    app.core(Update::Bus(names(&["tts"])));
    replied(&mut app, "答完了。");
    let ended = |reason| Update::Push(Push::TurnEnded(reason));
    app.voice_turn_ended(&ended(EndReason::Completed));
    assert!(calls(&mut sent).is_empty(), "没开自动朗读");
    app.auto_read();
    assert!(app.link.voice.auto_read);
    app.voice_turn_ended(&ended(EndReason::Interrupted));
    assert!(calls(&mut sent).is_empty(), "打断的不念");
    app.voice_turn_ended(&ended(EndReason::Completed));
    assert_eq!(calls(&mut sent)[0].1["params"]["text"], "答完了。");
}
