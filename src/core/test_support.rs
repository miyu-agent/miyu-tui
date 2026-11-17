//! 核心这一头的测试夹具。

use tokio::sync::mpsc;

use super::{Command, Core};

/// 不连核心的一头：发的命令收在交回的那一头里，测试照它看发了什么。
pub fn detached() -> (Core, mpsc::UnboundedReceiver<Command>) {
    let (core, sent, _) = detached_with_starts();
    (core, sent)
}

/// 同 [`detached`]，另交回「启动」那一头：测试照它看点没点「启动」。
pub fn detached_with_starts() -> (
    Core,
    mpsc::UnboundedReceiver<Command>,
    mpsc::UnboundedReceiver<()>,
) {
    let (commands, sent) = mpsc::unbounded_channel();
    let (starts, started) = mpsc::unbounded_channel();
    (Core { commands, starts }, sent, started)
}

/// 照流出来时读一串事件（`push.rs`），都不算这个界面发的。
pub fn read_live(events: &[serde_json::Value]) -> Vec<super::Push> {
    events
        .iter()
        .flat_map(|e| super::push::read(e, &|_| false))
        .collect()
}

/// 照补发的读一串事件（`replay.rs`），读完收尾。
pub fn read_replayed(events: &[serde_json::Value]) -> Vec<super::Push> {
    let mut replay = super::replay::Replay::default();
    let mut out: Vec<_> = events.iter().flat_map(|e| replay.read(e)).collect();
    out.extend(replay.finish());
    out
}
