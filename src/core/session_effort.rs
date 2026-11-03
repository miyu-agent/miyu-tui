//! 这个会话的思考强度（`/effort`，核心 SM-1；蓝图 `tui.md`「配置与模型」）：开着会话的发 `session.configure {effort}`，
//! 成了交回界面写底栏，被拒的照一般的拒绝弹；还没开会话的记着、当场算换成了，开了会话再补上。

use std::collections::HashMap;

use serde_json::json;

use super::awaiting::Awaiting;
use super::link::Link;
use super::rpc::Rpc;
use super::update::Update;

/// 换成 `level`（`None` 是跟着配置）。发不出去的交回错（连接断了）；`notify` 交回假的是界面退了。
pub(super) async fn configure(
    rpc: &mut Rpc,
    (link, awaiting): (&mut Link, &mut HashMap<String, Awaiting>),
    level: Option<String>,
    notify: &impl Fn(Update) -> bool,
) -> std::io::Result<bool> {
    let Some(session) = link.target() else {
        link.pending_effort = Some(level.clone());
        return Ok(notify(Update::EffortSet(level)));
    };
    let params = json!({"session": session, "effort": level});
    awaiting.insert(
        rpc.send("session.configure", params).await?,
        Awaiting::EffortSet(level),
    );
    Ok(true)
}

/// 刚开好的会话：没开时选过的补上。
pub(super) async fn flush(rpc: &mut Rpc, link: &mut Link, session: &str) -> std::io::Result<()> {
    if let Some(effort) = link.pending_effort.take() {
        let params = json!({"session": session, "effort": effort});
        rpc.send("session.configure", params).await?;
    }
    Ok(())
}
