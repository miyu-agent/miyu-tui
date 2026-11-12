//! 语音的按键、请求和回应（核心设计 20 第七节，蓝图 `tui.md`「语音」；状态和字在 `crate::voice`）：点麦克风、`/voice-input`
//! 开始听（`stt.record`），再点、再打一次停（`stt.stop`），录完交回的字插进输入框、不自动发；`/read-aloud` 念她上一轮的回答
//! （`tts.play`），在念的再打一次停（`tts.stop`）；`/auto-read` 开关自动朗读，开着时她每答完一轮就念。都经 `bus.call`，只认
//! `stt`、`tts` 两个名字。声音的进出都在核心那台机器上（经 ssh 连远端核心的是远端的麦克风、音箱）。

use serde_json::{Value, json};

use super::App;
use crate::core::{Command, EndReason, Push, Update};
use crate::oobe::asker::TAG_BASE;
use crate::voice::{Mic, heard, said};

/// `stt.record` 那一次：一直到录完、认完才回。
const RECORD_TAG: u64 = TAG_BASE - 40;
/// `tts.play` 那一次：念完、被停才回。
const PLAY_TAG: u64 = TAG_BASE - 41;
/// `stt.stop`、`tts.stop`：回应不用看。
const STOP_TAG: u64 = TAG_BASE - 42;

impl App {
    /// 总线上可用的名字变了（核心 BUS-2）：命令照它列，麦克风照它画。录着、念着的那一次照样等它回来。
    pub(super) fn voice_bus(&mut self, names: Vec<String>) {
        self.config.commands.bus.clone_from(&names);
        self.link.voice.ready = names;
    }

    /// 照名字调总线上的一个方法。
    fn bus_call(&mut self, tag: u64, name: &str, method: &str, params: Value) {
        self.link.core.send(Command::Ask {
            tag,
            method: "bus.call",
            params: json!({"name": name, "method": method, "params": params}),
        });
    }

    /// 点麦克风、`/voice-input`：不在听的开始听，在听的停（停了等字回来）；`stt` 不可用的不做什么（麦克风不画、命令不列）。
    pub(super) fn voice_input(&mut self) {
        if !self.link.voice.stt() {
            return;
        }
        match self.link.voice.mic {
            Mic::Idle => {
                self.bus_call(RECORD_TAG, "stt", "record", json!({}));
                self.link.voice.mic = Mic::Listening;
            }
            Mic::Listening => {
                self.bus_call(STOP_TAG, "stt", "stop", json!({}));
                self.link.voice.mic = Mic::Recognizing;
            }
            Mic::Recognizing => {}
        }
    }

    /// `/read-aloud`：念她上一轮的回答；在念的停掉。
    pub(super) fn read_aloud(&mut self) {
        if self.link.voice.speaking {
            self.bus_call(STOP_TAG, "tts", "stop", json!({}));
            return;
        }
        match self.talk.transcript.last_reply() {
            Some(text) => self.speak(text),
            None => {
                let note = self.config.text.voice.nothing_to_read.clone();
                self.hint(note, false);
            }
        }
    }

    /// 念一段回答（Markdown，`say` 自己转成念得出来的话）：带上这个会话的人格，照它挑音色。
    fn speak(&mut self, text: String) {
        let mut params = json!({ "text": text });
        if let Some(persona) = self.talk.transcript.persona.clone() {
            params["persona"] = json!(persona);
        }
        self.bus_call(PLAY_TAG, "tts", "play", params);
        self.link.voice.speaking = true;
    }

    /// `/auto-read`：开关自动朗读，只在这个终端里、不存。
    pub(super) fn auto_read(&mut self) {
        let on = !self.link.voice.auto_read;
        self.link.voice.auto_read = on;
        let note = self.config.text.voice.auto[usize::from(!on)].clone();
        self.hint(note, false);
    }

    /// 正在看的会话一轮照常答完：自动朗读开着、`tts` 可用的念这一轮的回答（打断的、出错的不念）。
    pub(super) fn voice_turn_ended(&mut self, update: &Update) {
        let done = matches!(update, Update::Push(Push::TurnEnded(EndReason::Completed)));
        if done
            && self.link.voice.auto_read
            && self.link.voice.tts()
            && let Some(text) = self.talk.transcript.last_reply()
        {
            self.speak(text);
        }
    }

    /// 语音那几次请求的回应归这里，交回 `true`：录完的字插进输入框（没听清的不放空字、说一句），出错的照出错码说一句。
    pub(super) fn voice_answer(&mut self, update: &Update) -> bool {
        let Update::Answer { tag, result } = update else {
            return false;
        };
        let texts = &self.config.text.voice;
        let note = match (*tag, result) {
            (RECORD_TAG, Ok(got)) => {
                self.link.voice.mic = Mic::Idle;
                match heard(got) {
                    Some(text) => {
                        self.typing.input.editor.insert(&text);
                        None
                    }
                    None => Some(texts.unclear.clone()),
                }
            }
            (RECORD_TAG, Err(refusal)) => {
                self.link.voice.mic = Mic::Idle;
                Some(said(refusal, texts))
            }
            (PLAY_TAG, got) => {
                self.link.voice.speaking = false;
                got.as_ref().err().map(|refusal| said(refusal, texts))
            }
            (STOP_TAG, _) => None,
            _ => return false,
        };
        if let Some(note) = note {
            self.hint(note, false);
        }
        true
    }
}

#[cfg(test)]
mod tests;
