//! 她说出来的语音（核心 SD-1，蓝图 `tui.md`「她说的语音」）：工具结果 `sent` 里写了 `"voice": true` 的那一项
//! `{blob, media_type, voice, duration_ms?, name?}`。放不放照这一轮是谁开的（核心 `view.md` 的 `voice` 条目，同网页
//! W-1212）：开这一轮的那条请求（`turn.started` 的 `cause`）是这个终端发的，当场放；开这一轮的那句话是到点叫醒的
//! （`by` 是 `schedule`）、这一轮不是哪句话开的，是没人开口的一轮，开着这个会话的都放；别的（别的头、群里的人）只画。
//! 这里只读出这几样（[`Spoken`]），记哪一轮谁开的、放不放在 `transcript/voice.rs`。

use serde_json::Value;

use super::Push;

/// 一段语音。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voice {
    /// 音频的内容哈希：放的时候照它交给 `tts.play`。
    pub blob: String,
    /// `audio/` 开头的媒体类型。
    pub media_type: String,
    /// 文件名；可以没有。
    pub name: Option<String>,
    /// 多长，毫秒；工具没报的没有。
    pub duration_ms: Option<u64>,
}

/// 语音条要的几样。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Spoken {
    /// 一句话落了盘（`message.user`）：序号，是不是到点叫醒的（`by` 是 `schedule`）。
    Asked {
        /// 序号。
        seq: u64,
        /// 到点叫醒的，不是人说的。
        nobody: bool,
    },
    /// 一轮开始了（`turn.started`）：开它的那句话的序号，开它的请求是不是这个终端发的。
    Opened {
        /// 开它的那句话（`trigger`）；不是哪句话开的是 `None`。
        trigger: Option<u64>,
        /// 开它的请求（`cause`）是这个终端发的。
        mine: bool,
    },
    /// 她说了一段。
    Clip(Voice),
}

/// `message.user` 的那一条。
pub(super) fn asked(event: &Value) -> Option<Push> {
    Some(Push::Voice(Spoken::Asked {
        seq: event["seq"].as_u64()?,
        nobody: event["by"]["kind"] == "schedule",
    }))
}

/// `turn.started` 的那一条。`mine`：这个请求编号是不是这个终端发的。
pub(super) fn opened(event: &Value, mine: &dyn Fn(&str) -> bool) -> Push {
    Push::Voice(Spoken::Opened {
        trigger: event["body"]["trigger"].as_u64(),
        mine: event["cause"].as_str().is_some_and(mine),
    })
}

/// 工具结果 `sent` 里的语音，照先后：没有 `blob`、不是 `audio/` 的不要。
pub(super) fn clips(sent: &Value) -> impl Iterator<Item = Push> + '_ {
    let text = |v: &Value| {
        v.as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    sent.as_array()
        .into_iter()
        .flatten()
        .filter(|item| item["voice"] == true)
        .filter_map(move |item| {
            let media_type = text(&item["media_type"]).filter(|m| m.starts_with("audio/"))?;
            Some(Push::Voice(Spoken::Clip(Voice {
                blob: text(&item["blob"])?,
                media_type,
                name: text(&item["name"]),
                duration_ms: item["duration_ms"].as_u64(),
            })))
        })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Spoken, Voice};
    use crate::core::push::{Push, read};

    #[test]
    fn a_voice_in_a_tool_result_and_who_opened_the_turn_are_read() {
        let mine = |c: &str| c == "tui-7";
        let result = json!({"seq": 9, "kind": "tool.result", "by": {"kind": "tool", "call_id": "c1"},
            "body": {"call_id": "c1", "status": "ok", "blocks": [{"type": "text", "text": "Spoken."}],
                "sent": [{"blob": "sha256:aa", "media_type": "audio/wav", "voice": true, "duration_ms": 1200,
                          "name": "say-1.wav"},
                         {"blob": "sha256:bb", "media_type": "image/png", "width": 2, "height": 2}]}});
        let voices: Vec<Push> = read(&result, &mine)
            .into_iter()
            .filter(|p| matches!(p, Push::Voice(_)))
            .collect();
        assert_eq!(
            voices,
            [Push::Voice(Spoken::Clip(Voice {
                blob: "sha256:aa".into(),
                media_type: "audio/wav".into(),
                name: Some("say-1.wav".into()),
                duration_ms: Some(1200),
            }))]
        );
        let started = json!({"seq": 5, "kind": "turn.started", "turn": 5, "cause": "tui-7",
            "by": {"kind": "kernel"}, "body": {"trigger": 4}});
        assert!(read(&started, &mine).contains(&Push::Voice(Spoken::Opened {
            trigger: Some(4),
            mine: true
        })));
        let woke = json!({"seq": 4, "kind": "message.user", "by": {"kind": "schedule", "schedule": "t1"},
            "cause": "schedule/t1/1", "body": {"blocks": [{"type": "text", "text": "到点了"}]}});
        assert!(read(&woke, &mine).contains(&Push::Voice(Spoken::Asked {
            seq: 4,
            nobody: true
        })));
    }
}
