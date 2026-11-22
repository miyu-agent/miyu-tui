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
    /// 循环到点交进来的那句照别处来的话画（灰竖线、暗字），来处写这个：「循环 · <那一句>」（2026-11-22 主人）。
    pub round: String,
    /// AI 自己挂的定时触发（核心 FX-21）到点交进来的那句，来处写这个。
    pub once: String,
    /// 光打 `/loop` 的循环交的是出厂的一句英文（`by.builtin`，核心 FX-15），不照原文写，写这一句。
    pub builtin: String,
    /// 看着的会话上的循环这一次只记下、没开轮（`last.result` 是 `held`，ACC-1121-#1）时底下提示的一句。
    pub held: String,
}

crate::from_json::record! { LoopTexts, deny { title, paused, next, dynamic, every, round, once, builtin, held } }

impl LoopTexts {
    /// 到点交进来的那句画成别处来的话时的来处和字：循环写 `round`、定时触发写 `once`；光打 `/loop` 的（没有原文）写 `builtin`。
    pub fn woke(&self, looping: bool, text: Option<String>) -> (String, String) {
        let from = if looping { &self.round } else { &self.once };
        (from.clone(), text.unwrap_or_else(|| self.builtin.clone()))
    }
}
