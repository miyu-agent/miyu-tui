//! 会话读不了的两种（核心 YC-2、YC-6；蓝图 `tui.md`「连核心」第 7 条下）：字在 `text/*.json` 的 `unread`。

use serde::Deserialize;

/// 会话读不了时的字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnreadTexts {
    /// 日志中间坏了一行，正文末尾写的：`{line}` 是第几行（核心 YC-6 的 `session_read_only`）。
    pub read_only: String,
    /// 同上，核心没说第几行。
    pub read_only_somewhere: String,
    /// 只读的会话，输入框空着时写的那一行：怎么新开会话。
    pub read_only_placeholder: String,
    /// 会话一时读不了（核心 YC-2 的 `session_unavailable`），过一会儿自动再试：`{wait}` 几秒以后。
    pub unavailable_retry: String,
    /// 再试了几次还是读不了：不再自动试。
    pub unavailable_gave_up: String,
}
