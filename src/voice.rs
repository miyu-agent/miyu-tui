//! 语音（核心设计 20 第七节「头」，蓝图 `tui.md`「语音」）：语音包经总线提供 `stt`（说话转字）、`tts`（念出来），终端只认这两个
//! 名字，不认包名。查到 `stt` 可用才画麦克风、列 `/voice-input`；查到 `tts` 可用才列 `/read-aloud`、`/auto-read`。录音、朗读都是
//! 调 `bus.call`，一次调用到录完、念完才回；这里放状态、给人看的字和读回应的几样，按键、请求在 `app/voice.rs`。

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::core::Refusal;

/// 给人看的字（`text/<语言>.json` 的 `voice`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Texts {
    /// 出错码（`error.data.code`，核心 `docs/reference/bus.md`）对应的一句；表里没有的照 `failed`。
    pub codes: HashMap<String, String>,
    /// 别的错：`{why}` 是核心的原话。
    pub failed: String,
    /// 没听清（`clear` 是假的）：不往输入框里放空字。
    pub unclear: String,
    /// `/read-aloud` 时还没有她的回答。
    pub nothing_to_read: String,
    /// `/auto-read` 打开、关上。
    pub auto: [String; 2],
    /// 自动朗读开着时底栏写的。
    pub auto_tag: String,
}

/// 麦克风那一格在哪一步：不在听（暗）、在听（红）、停了在等字（转圈）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Mic {
    /// 不在听。
    #[default]
    Idle,
    /// 发了 `stt.record`、还没回：在听。
    Listening,
    /// 人按了停（发了 `stt.stop`），等 `record` 交回字。
    Recognizing,
}

/// 语音的状态：总线上哪些名字可用、麦克风在哪一步、在不在念、自动朗读开没开（只在这个终端里，不存）。
#[derive(Debug, Clone, Default)]
pub struct Voice {
    /// `bus.list`、`bus.changed` 里 `ready` 的名字。
    pub ready: Vec<String>,
    /// 麦克风。
    pub mic: Mic,
    /// 发了 `tts.play` 还没回：在念。
    pub speaking: bool,
    /// 她每一轮答完自动念。
    pub auto_read: bool,
}

impl Voice {
    /// 能说话代替打字。
    pub fn stt(&self) -> bool {
        self.ready.iter().any(|n| n == "stt")
    }

    /// 能念出来。
    pub fn tts(&self) -> bool {
        self.ready.iter().any(|n| n == "tts")
    }
}

/// 读 `bus.list` 的回应、`bus.changed` 的参数（`{"names": [{"name", "kind", "package", "ready"}…]}`）：可用的名字。
pub fn ready(names: &Value) -> Vec<String> {
    let mut out: Vec<String> = names["names"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|n| n["ready"] == true)
        .filter_map(|n| n["name"].as_str().map(str::to_string))
        .collect();
    out.dedup();
    out
}

/// `stt.record` 交回的字：没听清的（`clear` 是假的、字是空的）是 `None`。
pub fn heard(result: &Value) -> Option<String> {
    let text = result["text"].as_str().map(str::trim).unwrap_or_default();
    (result["clear"] != false && !text.is_empty()).then(|| text.to_string())
}

/// 出错时说的一句：照 `data.code` 挑表里的；表里没有的照 `failed` 写核心的原话。
pub fn said(refusal: &Refusal, texts: &Texts) -> String {
    // 包自己的错在 `data.code`；没有包提供（`no_provider`）这类总线的错是原因码本身。
    let code = refusal.data["code"]
        .as_str()
        .or(refusal.reason.as_deref())
        .unwrap_or_default();
    match texts.codes.get(code) {
        Some(line) => line.clone(),
        None => texts.failed.replace("{why}", &refusal.message),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Voice, heard, ready, said};
    use crate::config::Config;
    use crate::core::Refusal;

    #[test]
    fn only_ready_names_count() {
        let names = json!({"names": [
            {"name": "stt", "kind": "provides", "package": "voice-input", "ready": false},
            {"name": "tts", "kind": "provides", "package": "say", "ready": true},
        ]});
        let voice = Voice {
            ready: ready(&names),
            ..Voice::default()
        };
        assert!(!voice.stt(), "模型还没下好的不算");
        assert!(voice.tts());
        assert!(ready(&json!({})).is_empty(), "旧核心、没有名字的");
    }

    #[test]
    fn an_unclear_take_puts_nothing_in_the_box() {
        assert_eq!(
            heard(&json!({"text": " 今天三点开会 ", "clear": true})),
            Some("今天三点开会".into())
        );
        assert_eq!(heard(&json!({"text": "", "clear": false})), None);
        assert_eq!(heard(&json!({"text": "嗯", "clear": false})), None);
    }

    #[test]
    fn each_code_says_its_own_line_and_the_rest_quote_the_core() {
        let texts = Config::builtin().unwrap().text.voice;
        let refusal = |code: &str| Refusal {
            reason: Some("method_failed".into()),
            message: "engine said no".into(),
            data: json!({"code": code}),
        };
        assert_eq!(said(&refusal("mic_busy"), &texts), "麦克风正被占用");
        assert_eq!(
            said(&refusal("whatever"), &texts),
            texts.failed.replace("{why}", "engine said no")
        );
        let no_provider = Refusal {
            reason: Some("no_provider".into()),
            message: "no provider".into(),
            data: json!({"name": "stt"}),
        };
        assert_eq!(said(&no_provider, &texts), "没有装语音软件包");
    }
}
