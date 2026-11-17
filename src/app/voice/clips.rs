//! 她说出来的语音怎么放（核心 SD-1，蓝图 `tui.md`「她说的语音」）：正文收下一段以后，该放的当场放（这个终端开的那一轮的；正在看的
//! 会话里没人开口的那一轮的，`transcript/voice.rs` 定）；点语音条放，在放的再点停。都经总线 `tts.play {"audio"}`、`tts.stop`，
//! 和 `/read-aloud` 一样从核心那台机器的音箱出来。放不了的（总线上没有 `tts`、终端不认的格式）不放，语音条上写着。

use serde_json::json;

use super::{App, STOP_TAG};
use crate::core::{Update, Voice};
use crate::oobe::asker::TAG_BASE;
use crate::transcript::Opener;
use crate::ui::voice_rows::{SCHEME, State};

/// 放语音的 `tts.play`：放完、被停才回。
const CLIP_TAG: u64 = TAG_BASE - 55;

impl App {
    /// 正在看的会话收完一条推送：刚来、该放的那一段当场放。
    pub(in crate::app) fn voice_arrived(&mut self) {
        if let Some((voice, _)) = self.talk.transcript.take_play() {
            self.play_clip(&voice);
        }
    }

    /// 停放着的会话收完一条推送、拿走了刚来的 `play`：只放这个终端开的那一轮的（没人开口的那一轮由开着它的头放，核心 `view.md`）。
    pub(in crate::app) fn parked_voice(&mut self, play: Option<(Voice, Opener)>) {
        if let Some((voice, Opener::Here)) = play {
            self.play_clip(&voice);
        }
    }

    /// 放一段：放不了的不放。在放别的的先停掉（`tts.stop` 连排着的一起停），不排在后面等。
    fn play_clip(&mut self, voice: &Voice) {
        let speaker = crate::ui::voice_rows::Speaker {
            ready: self.link.voice.tts(),
            playing: self.link.voice.clip.as_deref(),
        };
        if speaker.state(voice) == State::Unplayable {
            return;
        }
        if self.link.voice.clip.is_some() {
            self.bus_call(STOP_TAG, "tts", "stop", json!({}));
        }
        self.bus_call(CLIP_TAG, "tts", "play", json!({ "audio": voice.blob }));
        self.link.voice.clip = Some(voice.blob.clone());
        self.link.voice.clips += 1;
    }

    /// 点了语音条（挂着 [`SCHEME`] 的那一格）：在放的停，别的放；不是的交回 `false`。
    pub(in crate::app) fn open_voice_link(&mut self, url: &str) -> bool {
        let Some(id) = url.strip_prefix(SCHEME).and_then(|n| n.parse::<u64>().ok()) else {
            return false;
        };
        let voice = self
            .talk
            .transcript
            .entries
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.voice.clone());
        let Some(voice) = voice else {
            return true;
        };
        if self.link.voice.clip.as_deref() == Some(voice.blob.as_str()) {
            self.bus_call(STOP_TAG, "tts", "stop", json!({}));
        } else {
            self.play_clip(&voice);
        }
        true
    }

    /// 放语音的那几次回应：都回了算没在放；出错的照出错码说一句（被停的回 `finished: false`，不算错）。归这里交回 `true`。
    pub(in crate::app) fn clip_answer(&mut self, update: &Update) -> bool {
        let Update::Answer { tag, result } = update else {
            return false;
        };
        if *tag != CLIP_TAG {
            return false;
        }
        let voice = &mut self.link.voice;
        voice.clips = voice.clips.saturating_sub(1);
        if voice.clips == 0 {
            voice.clip = None;
        }
        if let Err(refusal) = result {
            let note = crate::voice::said(refusal, &self.config.text.voice);
            self.hint(note, false);
        }
        true
    }
}
