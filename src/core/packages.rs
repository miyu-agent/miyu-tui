//! 软件包列表的推送（核心 F-8 三补、9-8 七补）：订阅 `packages` 流，只听「工具目录变了」：装、卸、改了包（`package.changed`，
//! 包里的脚本、MCP 服务跟着变），个人设置里的 MCP 服务（`mcp.changed`）。变了重取给人看的字（`human.get`），新工具的显示名
//! 跟着换。列表本身不用（软件包页另外 `package.list`）。

use std::io;

use serde_json::{Value, json};

use super::rpc::Rpc;

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

    use super::tools_changed;

    #[test]
    fn packages_and_mcp_servers_changing_mean_the_tools_changed() {
        let push = |method: &str| json!({"method": method, "params": {}});
        assert!(tools_changed(&push("package.changed")));
        assert!(tools_changed(&push("mcp.changed")));
        assert!(!tools_changed(&push("sessions.changed")));
        assert!(!tools_changed(&push("config.changed")));
    }
}
