//! 「文件修改」的字（`text/zh.json` 的 `files`；蓝图 `tui.md`「文件修改」，发版前 check 9）。

use serde::Deserialize;

/// 「文件修改」的字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilesTexts {
    /// 侧边栏那一块的小标题、那一页上边框的标题。
    pub title: String,
    /// 那一页上边框标题后面：`{n}` 几个文件。
    pub count: String,
    /// 侧边栏放不下的：`{n}` 还有几个。
    pub more: String,
    /// `/files` 时这个会话还没改过文件。
    pub none: String,
    /// 那一页下面的按键提示：一条一个 `[键, 做什么]`。
    pub hints: Vec<[String; 2]>,
}
