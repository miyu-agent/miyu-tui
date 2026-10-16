//! 软件包列表的推送（核心 F-8 三补、9-8 七补）：订阅 `packages` 流，只听「工具目录变了」：装、卸、改了包（`package.changed`，
//! 包里的脚本、MCP 服务跟着变），个人设置里的 MCP 服务（`mcp.changed`）。变了重取给人看的字（`human.get`）和软件包列表
//! （`package.list`），新工具的显示名、包的图标跟着换。

use std::io;
use std::path::PathBuf;

use serde_json::{Value, json};

use super::rpc::Rpc;

/// `package.list` 里的一个包，终端用到的几格。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageEntry {
    /// 包的编号。
    pub id: String,
    /// 放状态的目录。
    pub state: Option<PathBuf>,
    /// 清单 `[package] icon` 写的 Lucide 图标名；没写的是 `None`。
    pub icon: Option<String>,
}

/// 读 `package.list` 的回应；没有编号的那一项不要。
pub(super) fn listed(result: &Value) -> Vec<PackageEntry> {
    result["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| {
            Some(PackageEntry {
                id: p["package"].as_str()?.to_string(),
                state: p["state"].as_str().map(PathBuf::from),
                icon: p["icon"]
                    .as_str()
                    .filter(|i| !i.is_empty())
                    .map(str::to_string),
            })
        })
        .collect()
}

/// 订阅 `packages` 流，交回请求编号；老核心不认的回错误，不用说（`take.rs`）。
pub(super) async fn follow(rpc: &mut Rpc) -> io::Result<String> {
    rpc.send("subscribe", json!({"stream": "packages"})).await
}

/// 这条推送说工具目录变了。
pub(super) fn tools_changed(message: &Value) -> bool {
    matches!(
        message["method"].as_str(),
        Some("package.changed" | "mcp.changed")
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{PackageEntry, listed, tools_changed};

    #[test]
    fn the_list_gives_each_package_its_state_and_icon() {
        let got = json!({"packages": [
            {"package": "tui", "state": "/s/tui"},
            {"package": "miyu-memes", "icon": "face-slightly-smiling"},
            {"package": "odd", "icon": ""},
            {"layer": "home"}
        ]});
        assert_eq!(
            listed(&got),
            [
                PackageEntry {
                    id: "tui".into(),
                    state: Some("/s/tui".into()),
                    icon: None,
                },
                PackageEntry {
                    id: "miyu-memes".into(),
                    state: None,
                    icon: Some("face-slightly-smiling".into()),
                },
                PackageEntry {
                    id: "odd".into(),
                    state: None,
                    icon: None,
                },
            ]
        );
    }

    #[test]
    fn packages_and_mcp_servers_changing_mean_the_tools_changed() {
        let push = |method: &str| json!({"method": method, "params": {}});
        assert!(tools_changed(&push("package.changed")));
        assert!(tools_changed(&push("mcp.changed")));
        assert!(!tools_changed(&push("sessions.changed")));
        assert!(!tools_changed(&push("config.changed")));
    }
}
