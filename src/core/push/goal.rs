//! 长期目标（蓝图 `tui.md`「长期目标」，核心 X-7、X-8）：它开的一轮。

use serde_json::Value;

use super::Push;

/// 长期目标发的那一句（`message.user`，`by` 是模块 `goal`）读成它开的一轮，序号是 `seq`；别人发的是 `None`。
pub(super) fn round(event: &Value, seq: u64) -> Option<Push> {
    let by = &event["by"];
    if by["kind"] != "module" || by["id"] != "goal" {
        return None;
    }
    let round = event["cause"].as_str().and_then(number);
    Some(Push::GoalRound { seq, round })
}

/// 核心造的命令编号 `goal-<第几轮>-<毫秒>` 里的第几轮（核心 X-8 补定的约定）；对不上这个写法的没有。
fn number(cause: &str) -> Option<u32> {
    let (round, ms) = cause.strip_prefix("goal-")?.split_once('-')?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(round) || !digits(ms) {
        return None;
    }
    round.parse().ok()
}
