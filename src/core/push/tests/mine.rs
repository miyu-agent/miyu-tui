//! 这几份测试共用的读法。

use super::super::{Push, Spoken, read};

/// 测试里的话都当是这个界面发的。
pub(in crate::core::push) fn read_mine(event: &serde_json::Value) -> Vec<Push> {
    plain(read(event, &|_| true))
}

/// 去掉语音条记账的那几条（谁开的这一轮，`push/voice.rs`；每句话、每一轮开头都有，别的测试不看它）。
pub(in crate::core::push) fn plain(pushes: Vec<Push>) -> Vec<Push> {
    pushes
        .into_iter()
        .filter(|p| !matches!(p, Push::Voice(Spoken::Asked { .. } | Spoken::Opened { .. })))
        .collect()
}
