//! 人用 `!` 打的命令（核心 SH-1，排期第 21 条；蓝图 `tui.md`「人自己跑命令 `!`」第 4 条）：`message.user` 多 `shell: true`，
//! 跑完一条 `shell.ran`（`message` 是那句话的序号）。

use serde_json::Value;

use super::Push;

/// 跑完了（`shell.ran`）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShellRan {
    /// 那句话的序号。
    pub message: u64,
    /// 退出码；被信号杀掉的、没跑完的是 `None`。
    pub exit: Option<i64>,
    /// 杀掉它的信号。
    pub signal: Option<i64>,
    /// 跑了多久（毫秒）。
    pub duration_ms: Option<u64>,
    /// 输出（标准输出和标准错误合在一起，核心截过的中间有一行写省了多少）。
    pub text: String,
    /// 异常结束的：`timed_out`、`interrupted`、`aborted`（认不得的照原样）。
    pub reason: Option<String>,
}

/// `message.user` 写了 `shell: true`：交回那句话是跑命令的（[`Push::ShellAsked`]）。
pub(super) fn asked(event: &Value) -> Option<Push> {
    (event["body"]["shell"] == true)
        .then(|| Push::ShellAsked(event["seq"].as_u64().unwrap_or_default()))
}

/// `shell.ran`。
pub(super) fn ran(body: &Value) -> Option<Push> {
    Some(Push::ShellRan(ShellRan {
        message: body["message"].as_u64()?,
        exit: body["exit"].as_i64(),
        signal: body["signal"].as_i64(),
        duration_ms: body["duration_ms"].as_u64(),
        text: body["text"].as_str().unwrap_or_default().to_string(),
        reason: body["reason"].as_str().map(str::to_string),
    }))
}
