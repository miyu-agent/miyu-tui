//! 循环的字（`text/zh.json` 的 `loops`；蓝图 `tui.md`「循环」）。

/// 循环的字。
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(serde::Deserialize, PartialEq))]
#[cfg_attr(test, serde(deny_unknown_fields))]
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
    /// 到点开下一轮的那句（`message.user` 的 `by` 是 `{kind: "schedule", loop}`）不照原文画，画暗色一行「循环 · 继续」
    /// （2026-11-17 项目主人验收第 3 条；核心 WK-2：不说「醒来」，说「继续」，字只在这里）。打头的钟照图标那一套的 `loop`
    /// （ACC-1121-#2）。
    pub round: String,
    /// 看着的会话上的循环这一次只记下、没开轮（`last.result` 是 `held`，ACC-1121-#1）时底下提示的一句。
    pub held: String,
}

crate::from_json::record! { LoopTexts, deny { title, paused, next, dynamic, every, round, held } }
