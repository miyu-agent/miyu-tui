//! 语音（核心设计 20 第七节「头」，蓝图 `tui.md`「语音」）：语音包经总线提供 `stt`（说话转字）、`tts`（念出来），终端只认这两个
//! 名字，不认包名。查到 `stt` 可用才画麦克风、列 `/voice-input`；查到 `tts` 可用才列 `/read-aloud`、`/auto-read`。录音、朗读都是
//! 调 `bus.call`，一次调用到录完、念完才回；这里放状态、给人看的字和读回应的几样，按键、请求在 `app/voice.rs`。

use std::collections::HashMap;

use serde_json::Value;

use crate::core::Refusal;

pub mod clips;

/// 给人看的字（`text/<语言>.json` 的 `voice`）。
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(serde::Deserialize, PartialEq))]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct Texts {
    /// 出错码（`error.data.code`，核心 `docs/zh/reference/bus.md`）对应的一句；表里没有的照 `failed`。
    pub codes: HashMap<String, String>,
    /// 别的错：`{why}` 是核心的原话。
    pub failed: String,
    /// 没听清（`clear` 是假的）：不往输入框里放空字。
    pub unclear: String,
    /// 没听到说话（语音包 VO-33：`speech` 是假的）：带设备名的（`{device}`）、没带的。
    pub no_speech: [String; 2],
    /// `/read-aloud` 时还没有她的回答。
    pub nothing_to_read: String,
    /// `/auto-read` 打开、关上。
    pub auto: [String; 2],
    /// 自动朗读开着时底栏写的。
    pub auto_tag: String,
    /// 她说的语音那一行（蓝图「她说的语音」）：打头的记号，没在放的、在放的。
    pub clip_marks: [String; 2],
    /// 工具没报多长的语音写的时长。
    pub clip_unknown: String,
    /// 放不了（没有能放声音的软件包、终端不认的格式）。
    pub unplayable: String,
}

crate::from_json::record! {
    Texts, deny {
        codes, failed, unclear, no_speech, nothing_to_read, auto, auto_tag, clip_marks,
        clip_unknown, unplayable,
    }
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
    /// 在放的那一段她说的语音（内容哈希，蓝图「她说的语音」）；没在放的是 `None`。
    pub clip: Option<String>,
    /// 发了几个放语音的 `tts.play` 还没回：都回了才算没在放（换一段放时先停的那一段后回）。
    pub clips: u32,
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

/// `stt.record` 没交回字时说的一句：没听到说话的（语音包 VO-33：`speech: false`，整段电平都够不上说话）提醒看看麦克风选对没有、
/// 带上现在录的那个（`device`，2026-11-17 项目主人验收第 22 条：系统默认输入是另一个麦克风，只录到环境声）；别的照「没听清」。
pub fn unheard(result: &Value, texts: &Texts) -> String {
    if result["speech"] != false {
        return texts.unclear.clone();
    }
    match result["device"]
        .as_str()
        .map(str::trim)
        .filter(|d| !d.is_empty())
    {
        Some(device) => texts.no_speech[0].replace("{device}", device),
        None => texts.no_speech[1].clone(),
    }
}

/// 出错时说的一句：照 `data.code` 挑表里的；表里没有的照 `failed` 写原话：包自己写的原因（`data.message`）在前，没写的照
/// 核心顶上的 `message`（总线的「方法执行失败」，ACC-1121-#4）。
pub fn said(refusal: &Refusal, texts: &Texts) -> String {
    // 包自己的错在 `data.code`；没有包提供（`no_provider`）这类总线的错是原因码本身。
    let code = refusal.data["code"]
        .as_str()
        .or(refusal.reason.as_deref())
        .unwrap_or_default();
    match texts.codes.get(code) {
        Some(line) => line.clone(),
        None => {
            let why = refusal.data["message"].as_str();
            let why = why.filter(|w| !w.trim().is_empty());
            texts
                .failed
                .replace("{why}", why.unwrap_or(&refusal.message))
        }
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
        // 语音包 VO-26：整段数字静音（2026-11-13 主人 1113 收听不到麦克风，查出是系统默认输入静音）
        assert!(said(&refusal("silent_audio"), &texts).starts_with("录到的全是静音"));
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

    /// 2026-11-20 主人本机验收：包的真原因在 `data.message`，顶上的 `message` 只是总线的「方法执行失败」。
    #[test]
    fn the_packages_own_reason_comes_before_the_bus_message() {
        let texts = Config::builtin().unwrap().text.voice;
        let refusal = |data| Refusal {
            reason: Some("method_failed".into()),
            message: "方法执行失败".into(),
            data,
        };
        let engine = refusal(json!({"code": "engine_failed", "message": "模型文件读不了"}));
        assert_eq!(
            said(&engine, &texts),
            texts.failed.replace("{why}", "模型文件读不了")
        );
        // 包没写原因的：照旧写总线的原话。
        let bare = refusal(json!({"code": "engine_failed"}));
        assert_eq!(
            said(&bare, &texts),
            texts.failed.replace("{why}", "方法执行失败")
        );
    }
}
