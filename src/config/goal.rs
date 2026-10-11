//! 长期目标的字（`text/zh.json` 的 `goal`；蓝图 `tui.md`「长期目标」）。

use serde::Deserialize;

/// 长期目标的字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalTexts {
    /// 它开的一轮那一行前面的记号，连同它后面的空格：暗色（第 4 条）。
    pub mark: String,
    /// 它开的一轮那一行，`{round}` 第几轮（第 4 条）。
    pub round: String,
    /// 它开的一轮那一行，认不出第几轮的。
    pub some_round: String,
    /// 还没开的会话里看、停、改目标：当场答（第 3 条）。
    pub none: String,
}
