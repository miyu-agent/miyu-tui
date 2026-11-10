//! 会话读不了的两种（核心 YC-2、YC-6；蓝图 `tui.md`「连核心」第 7 条下）：读页、订阅被拒时认出来，交给界面另外办，不当一般的拒绝。
//!
//! - `session_read_only`：日志中间坏了一行。最新一页照坏行以前的交过了（带 `broken`），订阅拒它：会话只能看。
//! - `session_unavailable`：载入、读日志时读写出错（权限、打开文件数用光），日志没坏：一时的错，过一会儿再试。

use std::io;

use serde_json::Value;

use super::awaiting::Awaiting;
use super::link::Link;
use super::rpc::Rpc;
use super::{Update, page, switch};

/// 读页（`page` 为真）、订阅 `session` 被拒：是这两种的交回界面要的那一条，别的 `None`（照一般的拒绝办）。
pub(super) fn refused(
    session: &str,
    reason: Option<&str>,
    data: &Value,
    page: bool,
) -> Option<Update> {
    match reason? {
        "session_read_only" => Some(Update::SessionReadOnly {
            session: session.to_string(),
            line: data["line"].as_u64(),
        }),
        "session_unavailable" => Some(Update::SessionUnavailable {
            session: session.to_string(),
            page,
        }),
        _ => None,
    }
}

/// 再试（[`super::Command::Reread`]）：从读最新一页起，或者只带读到的序号再订阅。交回请求编号和等它的那一种。
pub(super) async fn reread(
    rpc: &mut Rpc,
    link: &mut Link,
    session: &str,
    from_page: bool,
) -> io::Result<(String, Awaiting)> {
    if from_page {
        let id = page::latest(rpc, link, session).await?;
        return Ok((id, Awaiting::Page(session.to_string())));
    }
    let id = switch::replay(rpc, link, session).await?;
    Ok((id, Awaiting::Replay(session.to_string())))
}
