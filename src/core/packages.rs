//! 软件包列表的推送（核心 F-8 三补、9-8 七补）：订阅 `packages` 流，只听「工具目录变了」：装、卸、改了包（`package.changed`，
//! 包里的脚本、MCP 服务跟着变），系统配置里的 MCP 服务（`mcp.changed`）。变了重取给人看的字（`human.get`）和软件包列表
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
    /// 给人看的名字（照连接的语言）；没有的是 `None`。
    pub name: Option<String>,
    /// 它在的组（核心 BS-1下上，出厂的十八个在「基础系统」里）：编号、名字（照连接的语言，回应顶上的 `groups` 查）；不在组里的、
    /// 旧核心的是 `None`。在几个组里的取第一个。
    pub group: Option<(String, String)>,
}

/// 读 `package.list` 的回应；没有编号的那一项不要。
pub(super) fn listed(result: &Value) -> Vec<PackageEntry> {
    let names: std::collections::HashMap<&str, &str> = result["groups"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|g| Some((g["id"].as_str()?, g["name"].as_str().or(g["id"].as_str())?)))
        .collect();
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
                name: p["name"].as_str().map(str::to_string),
                group: p["groups"][0].as_str().map(|g| {
                    let name = names.get(g).copied().unwrap_or(g);
                    (g.to_string(), name.to_string())
                }),
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

/// 装包的下载进度：只推给发起装的那个连接，开始、结束各一次，中间最多 200 毫秒一次（核心 G-4 上）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    /// 哪个包。
    pub package: String,
    /// 下了多少字节。
    pub done: u64,
    /// 一共多少字节。
    pub total: u64,
}

/// 读 `package.progress {"package", "done", "total"}`；别的推送是 `None`。
pub(super) fn progress(message: &Value) -> Option<super::Update> {
    if message["method"] != "package.progress" {
        return None;
    }
    let params = &message["params"];
    Some(super::Update::PackageProgress(Progress {
        package: params["package"].as_str()?.to_string(),
        done: params["done"].as_u64().unwrap_or(0),
        total: params["total"].as_u64().unwrap_or(0),
    }))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{PackageEntry, listed, progress, tools_changed};
    use crate::core::Update;

    #[test]
    fn download_progress_is_read_from_its_own_notification() {
        let push = json!({"method": "package.progress",
            "params": {"package": "miyu-embed", "done": 1024, "total": 4096}});
        let want = super::Progress {
            package: "miyu-embed".into(),
            done: 1024,
            total: 4096,
        };
        assert!(matches!(progress(&push), Some(Update::PackageProgress(got)) if got == want));
        assert!(progress(&json!({"method": "package.changed", "params": {}})).is_none());
    }

    #[test]
    fn the_list_gives_each_package_its_state_and_icon() {
        let got = json!({"packages": [
            {"package": "miyu-tui", "state": "/s/miyu-tui"},
            {"package": "miyu-memes", "icon": "face-slightly-smiling"},
            {"package": "odd", "icon": ""},
            {"layer": "home"}
        ]});
        assert_eq!(
            listed(&got),
            [
                PackageEntry {
                    id: "miyu-tui".into(),
                    state: Some("/s/miyu-tui".into()),
                    icon: None,
                    name: None,
                    group: None,
                },
                PackageEntry {
                    id: "miyu-memes".into(),
                    state: None,
                    icon: Some("face-slightly-smiling".into()),
                    name: None,
                    group: None,
                },
                PackageEntry {
                    id: "odd".into(),
                    state: None,
                    icon: None,
                    name: None,
                    group: None,
                },
            ]
        );
    }

    #[test]
    fn a_package_in_a_group_carries_the_groups_name() {
        // 核心 BS-1下上：回应顶上列组，每一项写它在哪几个组里。
        let got = json!({"groups": [{"id": "basesystem", "name": "基础系统"}], "packages": [
            {"package": "shell", "name": "跑命令", "groups": ["basesystem"]},
            {"package": "odd", "groups": ["lonely"]},
            {"package": "weather"}
        ]});
        let groups: Vec<_> = listed(&got).into_iter().map(|p| p.group).collect();
        assert_eq!(
            groups,
            [
                Some(("basesystem".into(), "基础系统".into())),
                Some(("lonely".into(), "lonely".into())),
                None
            ],
            "顶上没列的组照编号写"
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
