//! 总线（核心 BUS-2，`docs/reference/bus.md`）：连上就订 `bus` 流，回应和 `bus.changed` 都是整份名字，照换；掉了队重新订。
//! 终端只看哪些名字可用（`ready`），语音照它画麦克风、列命令（`crate::voice`，设计 20）。

use serde_json::{Value, json};

use super::rpc::Rpc;

/// 订阅 `bus` 流，交回请求编号；老核心不认的回错误，不用说（`take.rs`）。
pub(super) async fn follow(rpc: &mut Rpc) -> std::io::Result<String> {
    rpc.send("subscribe", json!({"stream": "bus"})).await
}

/// `bus.changed`：可用的名字（整份）；别的推送是 `None`。
pub(super) fn changed(message: &Value) -> Option<Vec<String>> {
    (message["method"] == "bus.changed").then(|| crate::voice::ready(&message["params"]))
}

/// 这一条说 `bus` 流掉了队。
pub(super) fn lost(message: &Value) -> bool {
    message["method"] == "resync" && message["params"]["stream"] == "bus"
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn a_change_carries_the_whole_list_and_a_resync_is_noticed() {
        let push = json!({"method": "bus.changed", "params": {"names": [
            {"name": "stt", "kind": "provides", "package": "voice-input", "ready": true}
        ]}});
        assert_eq!(super::changed(&push), Some(vec!["stt".to_string()]));
        assert_eq!(super::changed(&json!({"method": "config.changed"})), None);
        assert!(super::lost(
            &json!({"method": "resync", "params": {"stream": "bus"}})
        ));
    }
}
