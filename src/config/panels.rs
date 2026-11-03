//! 贴在输入框上面的几块面板上的字（`text/zh.json`；蓝图 `tui.md`「输入历史列表」）。

use serde::Deserialize;

/// 输入历史列表上的字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryTexts {
    /// 标题。
    pub title: String,
    /// 标题后面的条数，`{count}`。
    pub count: String,
    /// 框里最下面搜索那一行没打字时暗色写的（「输入历史列表」第 1 条）。
    pub placeholder: String,
    /// 一条都对不上时那一行。
    pub empty: String,
    /// 好几行的，第一行后面跟着的，`{count}` 还有几行。
    pub lines: String,
    /// 预览太长时最后一行，`{count}` 还有几行。
    pub more: String,
    /// 最后一行按键提示。
    pub hints: String,
    /// 一分钟以内发的。
    pub now: String,
    /// `{n}` 分钟前发的。
    pub minutes: String,
    /// `{n}` 小时前发的。
    pub hours: String,
    /// `{n}` 天前发的（重启以后还在，「输入历史列表」第 8 条）。
    pub days: String,
    /// 还没发过话时按 Ctrl+R 的提示。
    pub none: String,
}

/// 斜杠命令列表上的字（蓝图「斜杠命令列表」第 3 条）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuTexts {
    /// 标题。
    pub title: String,
    /// 标题后面的条数，`{count}`。
    pub count: String,
}

/// 框下面那一行用量那一格的字（蓝图「框下面那一行」用量那一格，反馈 1102 第 22 条）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageCellTexts {
    /// 卡住：最近一次压缩失败、自动压缩暂停了，用量到了压缩线。
    pub failed: String,
    /// 卡住：用量大于窗口。
    pub over: String,
    /// 卡住时跟在后面那个能点的。
    pub compact: String,
    /// 正在压缩。
    pub compacting: String,
}
