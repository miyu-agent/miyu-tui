use crate::core::{Push, Update};

use super::Turn;
use crate::shellhook::say::look::Look;

fn said(seq: u64, text: &str) -> Update {
    Update::Push(Push::Said {
        seq,
        text: text.into(),
        attached: Vec::new(),
    })
}

#[test]
fn only_the_turn_this_sentence_started_counts() {
    // 接上的会话先补发最新一页：以前同一句话开的、别的话开的那几轮都不算（图纸「say」第 3 条）。
    let mut turn = Turn::new("你好".into(), None, Look::new(None));
    // 发出去以前推来的（补发的那一页里同样的话）不认。
    turn.watch(&said(1, "你好"));
    turn.watch(&Update::Push(Push::TurnStarted(1, Some(1))));
    assert_eq!(turn.ours, None, "发出去以前的");
    turn.sent = true;
    turn.watch(&said(2, "别的话"));
    turn.watch(&Update::Push(Push::TurnStarted(3, Some(2))));
    assert_eq!(turn.ours, None, "别的话开的那一轮");
    turn.watch(&said(9, "你好"));
    turn.watch(&Update::Push(Push::TurnStarted(10, Some(8))));
    assert_eq!(turn.ours, None, "trigger 不是这句话");
    turn.watch(&Update::Push(Push::TurnStarted(11, Some(9))));
    assert_eq!(turn.ours, Some(11));
}

#[test]
fn a_sentence_sent_on_this_connection_comes_back_as_a_bare_sequence() {
    // 接上已有的会话再发：自己发的那句推回来是 `user_message`，不带字（真模型实测，2026-10-20）。
    let mut turn = Turn::new("你好".into(), None, Look::new(None));
    turn.sent = true;
    turn.watch(&Update::Push(Push::UserMessage(32)));
    turn.watch(&Update::Push(Push::TurnStarted(33, Some(32))));
    assert_eq!(turn.ours, Some(33));
}

#[test]
fn the_sentence_waits_for_the_terminal_session_before_it_is_sent() {
    let turn = Turn::new("你好".into(), Some("/srv".into()), Look::new(None));
    assert_eq!(
        turn.pending.as_deref(),
        Some("你好"),
        "问到终端会话、接上以后才发"
    );
    assert!(!turn.done);
}

#[test]
fn a_sentence_heard_by_a_running_turn_belongs_to_that_turn() {
    // 回报叫醒她的那一轮还在跑的时候说话：这句话排着，被那一轮听到了，不另开一轮（2026-10-20 项目主人验收：终端没还回来）。
    let mut turn = Turn::new("起一个后台任务".into(), None, Look::new(None));
    // 那一轮在补发的那一页里开的、还没结束：发出去以前就记着。
    turn.watch(&Update::Push(Push::TurnStarted(319, Some(315))));
    turn.sent = true;
    turn.watch(&Update::Push(Push::UserMessage(326)));
    assert_eq!(turn.ours, None);
    turn.watch(&Update::Push(Push::Heard(320)));
    assert_eq!(turn.ours, None, "听到的还不到这一句");
    turn.watch(&Update::Push(Push::Heard(326)));
    assert_eq!(turn.ours, Some(319));
}

#[test]
fn the_turn_woken_right_after_ours_is_followed_but_not_one_someone_else_started() {
    // 后台命令在这一轮里跑完、她没顾上看：核心在这一轮结束的同一刻叫醒她再开一轮，接着印（2026-10-20 实测：没人印）。
    let mut turn = Turn::new("后台跑 sleep 3".into(), None, Look::new(None));
    turn.sent = true;
    turn.watch(&Update::Push(Push::UserMessage(367)));
    turn.watch(&Update::Push(Push::TurnStarted(368, Some(367))));
    turn.watch(&Update::Push(Push::Reported(379)));
    turn.unseen.ended(true, crate::core::JobReason::Finished);
    turn.watch(&Update::Push(Push::TurnEnded(
        crate::core::EndReason::Completed,
    )));
    turn.lingering = true;
    // 别处有人说了话开的那一轮不接。
    turn.watch(&Update::Push(Push::TurnStarted(381, Some(380))));
    assert!(turn.lingering, "人说话开的那一轮");
    turn.watch(&Update::Push(Push::TurnEnded(
        crate::core::EndReason::Completed,
    )));
    turn.watch(&Update::Push(Push::TurnStarted(383, Some(379))));
    assert!(!turn.lingering);
    assert_eq!(turn.woken, vec![383]);
    assert_eq!(turn.ours, Some(368), "从这句话开的那一轮印起");
    turn.ended = None;
    turn.watch(&Update::Push(Push::TurnEnded(
        crate::core::EndReason::Completed,
    )));
    assert_eq!(turn.ended, Some(crate::core::EndReason::Completed));
}

#[test]
fn the_done_line_carries_how_fast_this_turn_wrote_like_the_full_screen() {
    // 2026-10-27 项目主人：终端的收尾行接了「· 200 tok/s」，shellhook 的没有；收尾行照同一份正文写（`transcript/speed.rs`）。
    let mut config = crate::config::Config::builtin().unwrap();
    let (core, _sent) = crate::core::detached();
    let mut turn = Turn::new("你好".into(), None, Look::new(None));
    turn.sent = true;
    for update in [
        Push::UserMessage(1),
        Push::TurnStarted(2, Some(1)),
        Push::Speed {
            output: 400,
            ms: 2000,
        },
        Push::TurnEnded(crate::core::EndReason::Completed),
    ] {
        turn.core(Update::Push(update), &mut config, &core);
    }
    let done = turn
        .transcript
        .entries
        .iter()
        .rfind(|e| e.kind == crate::transcript::Kind::Done)
        .expect("有收尾行");
    assert!(done.text.ends_with(" · 200 tok/s"), "{}", done.text);
}

/// 一轮里她说了一段语音：开它的那句话（`asked`：到点叫醒的是真）、开它的请求是不是这条连接发的。
fn spoke(asked: Option<bool>, mine: bool) -> Vec<Update> {
    use crate::core::Spoken;
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
        Push::Voice(Spoken::Clip(crate::core::Voice {
            blob: "sha256:aa".into(),
            media_type: "audio/wav".into(),
            name: Some("say-1.wav".into()),
            duration_ms: Some(1200),
        })),
    ]);
    pushes.into_iter().map(Update::Push).collect()
}

#[test]
fn a_voice_plays_through_the_bus_only_in_a_turn_this_shell_or_nobody_opened() {
    // 2026-11-18 项目主人定：shell 里也照全屏放（`turn/clips.rs`）；别处开的只画，总线上没有 `tts` 的写「无法播放」。
    use crate::core::Command;
    use crate::ui::voice_rows::State;
    let cases = [
        (Some(false), true, true, true),
        (Some(true), false, true, true),
        (Some(false), false, true, false),
        (Some(false), true, false, false),
    ];
    for (asked, mine, tts, plays) in cases {
        let mut config = crate::config::Config::builtin().expect("出厂配置读得懂");
        let (core, mut sent) = crate::core::detached();
        let mut turn = Turn::new("念一句".into(), None, Look::new(None));
        let ready = if tts {
            vec!["tts".to_string()]
        } else {
            Vec::new()
        };
        turn.core(Update::Bus(ready), &mut config, &core);
        for update in spoke(asked, mine) {
            turn.core(update, &mut config, &core);
        }
        let calls: Vec<serde_json::Value> = std::iter::from_fn(|| sent.try_recv().ok())
            .filter_map(|command| match command {
                Command::Ask {
                    method: "bus.call",
                    params,
                    ..
                } => Some(params),
                _ => None,
            })
            .collect();
        let want: Vec<serde_json::Value> = if plays {
            vec![
                serde_json::json!({"name": "tts", "method": "play", "params": {"audio": "sha256:aa"}}),
            ]
        } else {
            Vec::new()
        };
        assert_eq!(calls, want, "{asked:?} {mine} {tts}");
        let clip = turn.transcript.entries.iter().find_map(|e| e.voice.clone());
        let clip = clip.expect("都画语音条");
        let state = turn.speaker().state(&clip);
        let drawn = match (tts, plays) {
            (false, _) => State::Unplayable,
            (true, true) => State::Playing,
            (true, false) => State::Idle,
        };
        assert_eq!(state, drawn, "{asked:?} {mine} {tts}");
    }
}

#[test]
fn a_finished_clip_is_drawn_idle_again() {
    let mut config = crate::config::Config::builtin().expect("出厂配置读得懂");
    let (core, _sent) = crate::core::detached();
    let mut turn = Turn::new("念一句".into(), None, Look::new(None));
    turn.core(Update::Bus(vec!["tts".into()]), &mut config, &core);
    for update in spoke(Some(false), true) {
        turn.core(update, &mut config, &core);
    }
    assert_eq!(turn.speaker().playing, Some("sha256:aa"));
    turn.core(
        Update::Answer {
            tag: 7,
            result: Ok(serde_json::json!({"finished": true})),
        },
        &mut config,
        &core,
    );
    assert_eq!(turn.speaker().playing, None);
}
