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
    /// 推演人生（蓝图「推演人生」）：人生那一行、人生卡、`/life` 跳转的列表、结束了的提示。
    pub life: LivesTexts,
}

/// 推演人生在会话里的字（`loops.life`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LivesTexts {
    /// 侧边栏那一行打头写的、人生卡的标题：「推演人生」。
    pub title: String,
    /// 人生卡一行一项的名字：人格、开始、下次继续、上一轮、状态。
    pub keys: [String; 5],
    /// 状态：进行中、已暂停。
    pub states: [String; 2],
    /// 按钮：暂停、继续、结束人生、确认结束。
    pub buttons: [String; 4],
    /// 按了一下「结束人生」以后，按钮上面一行写清后果。
    pub kill_hint: String,
    /// 人生卡下边框的按键提示。
    pub card_hint: String,
    /// 光打 `/life` 开的跳转列表：标题、下边框的按键提示。
    pub jump: [String; 2],
    /// 在跑的接下次几点，`{time}`。
    pub next: String,
    /// 这会儿正在跑一轮。
    pub busy: String,
    /// 一条没结束的人生都没有（还没开会话时光打 `/life`）。
    pub none: String,
    /// 她自己结束了的人生（核心 LF-14）：系统通知、输入框上面那一行「{name} 的人生结束了」，还有别的没看过的「（还有 {n} 段）」，
    /// 点得动的「打开」「收起」。
    pub ended: [String; 4],
}
