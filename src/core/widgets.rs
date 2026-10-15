//! 界面块的流（核心 `widgets.md`「头」，施工 X-15 中）：照界面说的屏上有哪几个位置订阅，记在连接上；重连、掉队照记着的
//! 重新订阅；推来的 `widget.changed` 一块一块交给界面。一个连接至多一个，换了说法就再订阅一次，核心换成新的。

use std::collections::HashMap;
use std::io;

use serde_json::{Value, json};

use super::Update;
use super::awaiting::Awaiting;
use super::link::Link;
use super::rpc::Rpc;
use crate::widgets::{View, block, blocks};

/// 照 `view` 订阅；`None` 是不再订阅。记进连接。
pub(super) async fn follow(
    rpc: &mut Rpc,
    link: &mut Link,
    view: Option<View>,
    awaiting: &mut HashMap<String, Awaiting>,
) -> io::Result<()> {
    link.widgets = view;
    match &link.widgets {
        Some(view) => {
            let id = rpc.send("subscribe", view.params()).await?;
            awaiting.insert(id, Awaiting::Widgets);
        }
        None => {
            rpc.send("unsubscribe", json!({"stream": "widgets"}))
                .await?;
        }
    }
    Ok(())
}

/// 连上了、掉了队：照记着的再订阅一次（没订阅过的不订）。
pub(super) async fn rejoin(
    rpc: &mut Rpc,
    link: &Link,
    awaiting: &mut HashMap<String, Awaiting>,
) -> io::Result<()> {
    if let Some(view) = &link.widgets {
        let id = rpc.send("subscribe", view.params()).await?;
        awaiting.insert(id, Awaiting::Widgets);
    }
    Ok(())
}

/// 订阅的回应：全部块。
pub(super) fn listed(result: &Value) -> Update {
    Update::Widgets(blocks(result))
}

/// 是 `widget.changed` 的交回那一条；别的是 `None`。
pub(super) fn changed(message: &Value) -> Option<Update> {
    if message["method"] != "widget.changed" {
        return None;
    }
    let params = &message["params"];
    Some(Update::WidgetChanged {
        package: params["package"].as_str()?.to_string(),
        id: params["id"].as_str()?.to_string(),
        block: block(&params["widget"]),
    })
}

/// 是界面块的流掉了队。
pub(super) fn lost(message: &Value) -> bool {
    message["method"] == "resync" && message["params"]["stream"] == "widgets"
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Update, changed, lost};

    #[test]
    fn a_change_is_read_and_a_removal_is_none() {
        let got = changed(&json!({"method": "widget.changed", "params": {
            "package": "wx", "id": "temp", "widget": null
        }}));
        assert!(
            matches!(got, Some(Update::WidgetChanged { ref package, ref id, block: None }) if package == "wx" && id == "temp"),
            "{got:?}"
        );
        let other = changed(&json!({"method": "config.changed", "params": {}}));
        assert!(other.is_none());
        assert!(lost(
            &json!({"method": "resync", "params": {"stream": "widgets"}})
        ));
        assert!(!lost(
            &json!({"method": "resync", "params": {"stream": "config"}})
        ));
    }
}
