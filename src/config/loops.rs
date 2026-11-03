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
}
