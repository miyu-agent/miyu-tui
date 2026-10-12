//! 长期目标（蓝图 `tui.md`「长期目标」，核心 X-7、X-8）：它开的一轮，它现在的样子。

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

/// 长期目标现在的样子（订阅回应里的 `goal`、推来的 `goal.changed`，核心 X-8 补）：只读界面要的几格。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Goal {
    /// 在哪一步。
    pub state: GoalState,
    /// 设下以后开过几轮。
    pub round: u32,
    /// 轮数上限；不限的没有。
    pub limit: Option<u32>,
}

/// 在哪一步（核心 `view.md`「会话状态」的 `goal.state`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalState {
    /// 跑着。
    Running,
    /// 在等人开口：续轮开的一轮没调工具。
    Waiting,
    /// 停着（核心重启以后载入的也是）。
    Paused,
    /// 卡住了。
    Blocked,
    /// 做完了。
    Completed,
    /// 认不得的（新版本才有的）。
    Other,
}

/// 读一份目标（`goal` 那一格）；`null`、没有、读不懂的是没有目标。
pub(crate) fn read(goal: &Value) -> Option<Goal> {
    let state = match goal["state"].as_str()? {
        "running" => GoalState::Running,
        "waiting" => GoalState::Waiting,
        "paused" => GoalState::Paused,
        "blocked" => GoalState::Blocked,
        "completed" => GoalState::Completed,
        _ => GoalState::Other,
    };
    let number = |v: &Value| v.as_u64().and_then(|n| u32::try_from(n).ok());
    Some(Goal {
        state,
        round: number(&goal["round"]).unwrap_or_default(),
        limit: number(&goal["limit"]),
    })
}
