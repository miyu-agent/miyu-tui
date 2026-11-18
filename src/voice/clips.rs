//! 放她说的语音（核心 SD-1，蓝图 `tui.md`「她说的语音」）：全屏（`app/voice/clips.rs`）和终端集成（`shellhook/say/clips.rs`）
//! 共用这一套：经总线 `tts.play {"audio"}` 放、`tts.stop` 停，放不了的（总线上没有 `tts`、终端不认的格式）不放，放着别的先停。
//! 放不放（谁开的这一轮）由 `transcript/voice.rs` 定，请求谁发、回应怎么说归各头。

use serde_json::{Value, json};

use super::Voice;
use crate::core::Command;
use crate::ui::voice_rows::{Speaker, State};

/// 放、停那两次请求的编号：各头自己挑，回应照它认。
#[derive(Debug, Clone, Copy)]
pub struct Tags {
    /// `tts.play`：放完、被停才回。
    pub play: u64,
    /// `tts.stop`：回应不用看。
    pub stop: u64,
}

/// 照名字调总线上的一个方法。
pub fn bus_call(tag: u64, name: &str, method: &str, params: Value) -> Command {
    Command::Ask {
        tag,
        method: "bus.call",
        params: json!({"name": name, "method": method, "params": params}),
    }
}

impl Voice {
    /// 画语音条用的：总线上有没有 `tts`、在放哪一段。
    pub fn speaker(&self) -> Speaker<'_> {
        Speaker {
            ready: self.tts(),
            playing: self.clip.as_deref(),
        }
    }

    /// 放 `clip` 要发的请求，记上在放：放不了的没有；在放别的先停（`tts.stop` 连排着的一起停），不排在后面等。
    pub fn play_clip(&mut self, clip: &crate::core::Voice, tags: Tags) -> Vec<Command> {
        if self.speaker().state(clip) == State::Unplayable {
            return Vec::new();
        }
        let mut out = Vec::new();
        if self.clip.is_some() {
            out.push(bus_call(tags.stop, "tts", "stop", json!({})));
        }
        out.push(bus_call(
            tags.play,
            "tts",
            "play",
            json!({ "audio": clip.blob }),
        ));
        self.clip = Some(clip.blob.clone());
        self.clips += 1;
        out
    }

    /// 放语音的 `tts.play` 回了一次：发出去的都回了才算没在放（换一段放时先停的那一段后回）。
    pub fn clip_answered(&mut self) {
        self.clips = self.clips.saturating_sub(1);
        if self.clips == 0 {
            self.clip = None;
        }
    }
}
