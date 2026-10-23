//! 人格和预设（核心 P-1 上、P-2 上，蓝图 `tui.md`「新会话：人格、工作区」「配置页」第 34 到 38 条）：
//! `persona.list`、`preset.list` 读成一行行，两样一个形状；新会话的框、配置页的两页、默认人格和默认预设的下拉都用它。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `preset.list` 里的一个预设：和人格一个形状。
pub type Preset = Persona;

/// `persona.list` 里的一个人格（`preset.list` 里的一个预设也是它）。先画后连时照上次记下的（`app/look.rs`）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Persona {
    /// 编号。
    pub id: String,
    /// 照这个连接的语言挑好的名字；没有的是 `None`。
    pub name: Option<String>,
    /// 照语言挑好的说明。
    pub summary: Option<String>,
    /// 文件写错了：核心的一句英文；能用的是 `None`。
    pub problem: Option<String>,
    /// 头像的版本（核心 P-5）：没有头像的、预设的是 `None`。
    pub avatar: Option<String>,
    /// 自选的主题色 `#rrggbb`（核心 P-6 的 `seed`）：没写的是 `None`，跟随头像的配色照头像的候选里打分最高的（「主题」第 5 条）。
    pub seed: Option<String>,
    /// 头像的候选主色 `#rrggbb`，好的在前（核心 P-6 补的 `seeds`，最多 4 个）：没有头像、解不开的是 `None`，全透明的是空的。
    pub seeds: Option<Vec<String>>,
}

impl Persona {
    /// 给人看的名字：没有的写编号。
    pub fn label(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }
}

/// 读 `persona.list` 的回应。
pub fn read_personas(got: &Value) -> Vec<Persona> {
    read_list(got, "personas", "persona")
}

/// 读 `preset.list` 的回应。末尾那一项「无工具」编号是 `null`（核心 P-16 下），读成空的编号，和「无人格」一个记法。
pub fn read_presets(got: &Value) -> Vec<Preset> {
    read_list(got, "presets", "preset")
}

/// 读 `preset.list` 的回应，去掉「无工具」：它不是文件，配置页不能编辑、不能设成默认（核心 P-16 下）。
pub fn read_preset_files(got: &Value) -> Vec<Preset> {
    let mut list = read_presets(got);
    list.retain(|p| !p.id.is_empty());
    list
}

/// 会话信息里用的预设（会话列表、`subscribe` 回应）：无工具的写 `preset: null`、`no_tools: true`，读成空的编号
/// （核心 P-16 下）；没写的是 `None`。
pub fn session_preset(info: &Value) -> Option<String> {
    match info["preset"].as_str() {
        Some(id) => Some(id.to_string()),
        None if info["no_tools"] == true => Some(String::new()),
        None => None,
    }
}

/// 开会话、换预设时交的预设：空的编号（无工具）交 `null`（核心 P-16 下）。
pub fn preset_param(id: &str) -> Value {
    if id.is_empty() {
        Value::Null
    } else {
        Value::from(id)
    }
}

/// 一行行在 `list` 那一格，编号在每一项的 `id` 那一格。
fn read_list(got: &Value, list: &str, id: &str) -> Vec<Persona> {
    let text = |v: &Value| v.as_str().map(str::to_string);
    got[list]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| Persona {
            id: text(&p[id]).unwrap_or_default(),
            name: text(&p["name"]),
            summary: text(&p["summary"]),
            problem: text(&p["problem"]),
            avatar: text(&p["avatar"]),
            seed: text(&p["seed"]),
            seeds: p["seeds"].as_array().map(|list| {
                list.iter()
                    .filter_map(|s| s.as_str().map(str::to_string))
                    .collect()
            }),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{preset_param, read_preset_files, read_presets, session_preset};

    /// 核心 P-16 下的 `preset.list`：末尾一项编号是 `null` 的「无工具」。
    fn listed() -> serde_json::Value {
        json!({"presets": [
            {"preset": "full", "name": "全部"},
            {"preset": null, "name": "无工具", "summary": "不提供任何工具。", "icon": "circle-off"},
        ]})
    }

    #[test]
    fn no_tools_reads_as_the_empty_id_and_is_left_out_of_the_files() {
        let all = read_presets(&listed());
        assert_eq!(all.len(), 2);
        assert_eq!(all[1].id, "");
        assert_eq!(all[1].label(), "无工具");
        let files = read_preset_files(&listed());
        assert_eq!(
            files.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["full"]
        );
    }

    #[test]
    fn a_session_with_no_tools_is_the_empty_id_and_is_sent_back_as_null() {
        assert_eq!(
            session_preset(&json!({"preset": "full"})).as_deref(),
            Some("full")
        );
        let none = json!({"preset": null, "no_tools": true});
        assert_eq!(session_preset(&none).as_deref(), Some(""));
        assert_eq!(session_preset(&json!({})), None, "以前的会话没记预设");
        assert_eq!(preset_param(""), json!(null));
        assert_eq!(preset_param("full"), json!("full"));
    }
}
