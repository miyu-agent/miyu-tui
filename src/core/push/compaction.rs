//! 压缩的几样（蓝图 `tui.md`「压缩、清空、回顾」）：从 `push.rs` 挪出来（那个文件到了 500 行）。

use super::CallError;

/// 压缩的几样：进度、压好了、摘要请求出错、暂停了自动压缩。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compaction {
    /// 摘要写到哪了（瞬时的 `compaction.progress`）：收到多少字。
    Progress {
        /// 收到的正文字数。
        written: u64,
        /// 核心估计要写多少字（压前的用量夹在 2 万到 8 万之间）；以前的核心不给。
        expected: Option<u64>,
        /// 手动压缩（`trigger` 是 `manual`）、自动的（`auto`）；以前的核心不给。
        manual: Option<bool>,
    },
    /// 提前在后台开始压了（瞬时的 `compaction.started`，核心 6-11 再补）。
    Started,
    /// 压好了（瞬时的 `compaction.done`）：压之前、压完的用量，都是估算。
    Done {
        /// 压之前的用量。
        before: u64,
        /// 压完的用量。
        after: u64,
        /// 换上的是提前在后台压好的那一份（`prepared: true`）。
        prepared: bool,
    },
    /// 摘要请求出错（`model.called` 带 `compaction`、`result` 是 `error`）。
    Failed(CallError),
    /// 暂停了自动压缩（`context.compaction_paused`，施工 6-6 上）。
    Paused {
        /// `failures`、`too_large`，认不得的照原样。
        reason: String,
        /// 连着失败了几次。
        failures: Option<u64>,
        /// 估得最大的那一条的序号。
        entry: Option<u64>,
    },
}
