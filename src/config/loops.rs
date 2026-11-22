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
    /// 到点开下一轮的那句（`message.user` 的 `by` 是 `{kind: "schedule", loop}`）不当你的话画，画暗色一行旁白
    /// 「循环 · {words}」，`{words}` 是这一轮交进来的那句的第一行（2026-11-22 项目主人：要看得出循环这次说了什么）。打头的钟
    /// 照图标那一套的 `loop`（ACC-1121-#2）。
    pub round: String,
    /// 光打 `/loop` 的循环交的是出厂的一句英文（`by.builtin`，核心 FX-15），不照原文写，写这一句。
    pub builtin: String,
    /// 看着的会话上的循环这一次只记下、没开轮（`last.result` 是 `held`，ACC-1121-#1）时底下提示的一句。
    pub held: String,
}

crate::from_json::record! { LoopTexts, deny { title, paused, next, dynamic, every, round, builtin, held } }

impl LoopTexts {
    /// 到点那一轮的旁白：带着这一轮交进来的那句的第一行写 `round`，光打 `/loop` 的（没有原文）写 `builtin`。
    pub fn round_line(&self, words: Option<String>) -> String {
        words.map_or_else(
            || self.builtin.clone(),
            |w| self.round.replace("{words}", &w),
        )
    }
}
