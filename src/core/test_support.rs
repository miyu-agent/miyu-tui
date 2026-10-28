//! 核心这一头的测试夹具。

use tokio::sync::mpsc;

use super::{Command, Core};

/// 不连核心的一头：发的命令收在交回的那一头里，测试照它看发了什么。
pub fn detached() -> (Core, mpsc::UnboundedReceiver<Command>) {
    let (commands, sent) = mpsc::unbounded_channel();
    (Core { commands }, sent)
}
