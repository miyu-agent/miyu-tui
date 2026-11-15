//! 人格的窗里的「后天」（核心 LF-11，设计 70 第六节）：这个人格在人生会话里自己记下的变化（记忆的 `change` 那一类），只读，
//! 旧的在前；每一条下面暗色写为什么变的（`why`）。人在这里只能看，作废、恢复在记忆那一页（核心说人改不了：`change_note`）。

use serde_json::{Value, json};

use crate::core::Refusal;
use crate::settings::{Settings, Waiting};

/// 记下的一处变化。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// 那一条。
    pub text: String,
    /// 为什么变的；没写的是 `None`。
    pub why: Option<String>,
}

/// 读 `memory.list {class: "change"}` 的回应：没作废的，旧的在前（核心交回的是新的在前）。
pub fn read(got: &Value) -> Vec<Change> {
    let mut out: Vec<Change> = got["memories"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|m| m["class"] == "change" && m["retired"].is_null())
        .filter_map(|m| {
            Some(Change {
                text: m["text"].as_str()?.to_string(),
                why: m["why"]
                    .as_str()
                    .filter(|w| !w.trim().is_empty())
                    .map(str::to_string),
            })
        })
        .collect();
    out.reverse();
    out
}

impl Settings {
    /// 开人格的窗时问一次这个人格记下的变化。
    pub(in crate::settings) fn ask_changes(&mut self, persona: &str) {
        let params = json!({"persona": persona, "class": "change"});
        self.ask("memory.list", params, Waiting::Changes(persona.to_string()));
    }

    /// 读到了；被拒的（没装记忆、旧核心）当没有，不弹。
    pub(in crate::settings) fn changes_answer(
        &mut self,
        persona: String,
        result: Result<Value, Refusal>,
    ) {
        let list = result.map(|got| read(&got)).unwrap_or_default();
        self.more.changes = Some((persona, list));
    }

    /// 这个人格记下的变化；还没读到、读不到的是空的。
    pub fn changes_of(&self, persona: &str) -> &[Change] {
        self.more
            .changes
            .as_ref()
            .filter(|(id, _)| id == persona)
            .map_or(&[], |(_, list)| list.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::read;

    #[test]
    fn changes_are_listed_oldest_first_without_retired_ones() {
        let got = read(&json!({"memories": [
            {"id": "m3", "class": "change", "text": "这阵子常看书到十二点", "why": "迷上了推理小说", "retired": null},
            {"id": "m2", "class": "change", "text": "已经作废的", "retired": "淡掉了"},
            {"id": "m1", "class": "change", "text": "开始喝无糖的", "retired": null},
            {"id": "m0", "class": "episode", "text": "别的类不算", "retired": null},
        ]}));
        let texts: Vec<&str> = got.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(texts, ["开始喝无糖的", "这阵子常看书到十二点"], "旧的在前");
        assert_eq!(got[0].why, None);
        assert_eq!(got[1].why.as_deref(), Some("迷上了推理小说"));
        assert!(read(&json!({})).is_empty());
    }
}
