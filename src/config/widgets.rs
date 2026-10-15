//! 界面块的版面（`layout.json` 的 `widgets`，蓝图「界面块」第 4 条）：每个位置几块、一块几行，列表的记号。

use serde::Deserialize;

/// 每个位置的上限和列表的记号。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WidgetLayout {
    /// 首页最多几块。
    pub home_blocks: usize,
    /// 首页一块最多几行（连标题）。
    pub home_rows: usize,
    /// 首页的块合起来最多几行（连块之间的空行）。
    pub home_total: usize,
    /// 侧边栏一块最多几行（连标题）。
    pub sidebar_rows: usize,
    /// 底栏最多几格。
    pub status_cells: usize,
    /// 底栏一格最多几列。
    pub status_width: usize,
    /// 列表一条前面的记号。
    pub marks: WidgetMarks,
}

/// 列表一条前面的记号，连同后面的空格（照 `state`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WidgetMarks {
    pub todo: String,
    pub doing: String,
    pub done: String,
    pub failed: String,
    pub off: String,
}
