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
    /// 输入框上边框右边那一截打头的词（第 5 条）：「目标 · 进行中 · 第 2/5 轮」的「目标」。
    pub label: String,
    /// 在跑。
    pub running: String,
    /// 在等人开口。
    pub waiting: String,
    /// 停着。
    pub paused: String,
    /// 卡住了。
    pub blocked: String,
    /// 第几轮，`{round}`。
    pub rounds: String,
    /// 有上限的第几轮，`{round}`、`{limit}`。
    pub rounds_limited: String,
    /// 做完了（只在浮窗里写）。
    pub completed: String,
    /// 点目标那一行开的浮窗（`ui/goal_box.rs`）：标题、下边框的按键提示。
    pub box_title: String,
    pub box_hint: String,
    /// 浮窗左边一列的名字：目标、状态、轮数、用时、用量。
    pub box_keys: [String; 5],
    /// 用量那一格，`{tokens}`。
    pub box_tokens: String,
}
