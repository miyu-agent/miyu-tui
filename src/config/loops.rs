//! 循环的字（`text/zh.json` 的 `loops`；蓝图 `tui.md`「循环」）。

use serde::Deserialize;

/// 循环的字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoopTexts {
    /// 侧边栏那一段的标题（第 3 条）。
    pub title: String,
    /// 停着。
    pub paused: String,
    /// 下一次，`{time}`。
    pub next: String,
    /// 动态间隔。
    pub dynamic: String,
    /// 固定间隔，`{n}`：秒、分钟、小时、天。
    pub every: [String; 4],
    /// 到点开下一轮的那句（`message.user` 的 `by` 是 `{kind: "schedule", loop}`）不照原文画，画暗色一行：记号连同后面的空格、
    /// 循环的、人生的（2026-11-17 项目主人验收第 3 条；核心 WK-2：不说「醒来」，说「继续」，字只在这里）。
    pub round: [String; 3],
}
