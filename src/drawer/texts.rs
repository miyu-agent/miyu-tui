//! 抽屉上的字（`resources/text/zh.json` 的 `drawer`，蓝图「确认和提问的抽屉」）。

use std::collections::HashMap;

use serde::Deserialize;

/// 抽屉上的字和记号。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Texts {
    /// 顶上一行谁在问，`{who}`。
    pub asking: String,
    /// 跑命令的确认不在沙盒里跑时写的那一行（核心 D-4）。
    pub unsandboxed: String,
    /// 跑命令的确认被认成高危命令时写的那一行（核心 5-17，2026-10-20 项目主人：「可以加『高危命令』四个字」）。
    pub hazard: String,
    /// 选项预览面板第一行。
    pub preview: String,
    /// 子会话问的，`{who}` 怎么写：`子代理 {name}`。
    pub agent: String,
    /// 别的会话（不是正在看的这棵树）问的：不弹，提示一句，`{title}` 是那个会话的标题（第 10 条）。
    pub elsewhere: String,
    /// 答过的标签后面接的。
    pub answered_tab: String,
    /// 最后一个标签：「确认」页。
    pub review_tab: String,
    /// 选中的那一项行首。
    pub pointer: String,
    /// 能多选的：没勾、勾上。
    pub unchecked: String,
    /// 见 `unchecked`。
    pub checked: String,
    /// 提问的最后一项。
    pub other: String,
    /// 补充那一行打头的。
    pub notes_label: String,
    /// 确认的几项，照 `Decision::ALL` 的先后。
    pub decisions: [String; 3],
    /// 写「不允许」的理由时，还没打字暗色写的。
    pub reason_hint: String,
    /// 光标停在「不允许」上、没在写时后面写的：怎么写理由。
    pub reason_key: String,
    /// 要跑的命令往上滚过了：上面还有几行，`{n}`。
    pub command_above: String,
    /// 要跑的命令放不下：下面还有几行，`{n}`。
    pub command_below: String,
    /// 命令能滚时按键提示里多的那一项。
    pub keys_scroll: String,
    /// 预览能滚时按键提示里多的那一项。
    pub keys_scroll_preview: String,
    /// 预览放不下时「预览」后面写的：下面还有几行，`{n}`。
    pub preview_below: String,
    /// 确认的问题行，照 `access` 找，`{count}` 路径数、`{tool}` 工具名。
    pub access: HashMap<String, String>,
    /// `access` 里没有的。
    pub access_other: String,
    /// 装、卸软件包，打开扩展的确认（核心 F-10 下）。
    pub package: super::PackageTexts,
    /// 工作区外的路径后面标的。
    pub outside: String,
    /// 别的程序的私密目录（`zone` 是 `private`，核心 5-13）后面标的。
    pub private: String,
    /// 受保护的路径（`zone` 是 `read_only`：`.git/hooks`、shell 启动文件这类，核心 5-17 下上）后面标的。
    pub protected: String,
    /// 「确认」页、留下的引用块里没答的那道。
    pub unanswered: String,
    /// 按键提示：提问单选、提问多选、确认、「确认」页、编辑时；几道题时提问的后面再接 `keys_tabs`。
    pub keys: String,
    /// 见 `keys`。
    pub keys_multi: String,
    /// 见 `keys`。
    pub keys_approve: String,
    /// 见 `keys`。
    pub keys_review: String,
    /// 见 `keys`。
    pub keys_edit: String,
    /// 写「不允许」的理由时：`Enter` 直接交。
    pub keys_reason: String,
    /// 见 `keys`。
    pub keys_tabs: String,
    /// 回答了以后正文里那一块的第一行，`{n}`：「你回答了 N 个问题」（`ui/answered_rows.rs`）。
    pub answered_head: String,
    /// 回答一道一行，`{label}` `{answer}`：「问题 → 答案」。
    pub answered_line: String,
    /// 补充的话接在那一行后面，`{notes}`。
    pub notes_suffix: String,
    /// 按过一下 `Esc` 以后，按键提示那一行换成的。
    pub cancel_hint: String,
    /// 不允许并写了理由，`{reason}`。
    pub denied_with: String,
}
