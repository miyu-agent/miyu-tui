//! shellhook 给人看的字：一种语言一份（`resources/shellhook/text/<代码>.json`），界面语言照终端界面的认法（蓝图 `tui.md`
//! 「界面语言」）。

use serde::Deserialize;

/// 每种语言的字，编译时带进来；和终端界面的 `text/` 一样多（测试查）。
pub const TEXTS: &[(&str, &str)] = &[
    ("zh", include_str!("../../resources/shellhook/text/zh.json")),
    ("en", include_str!("../../resources/shellhook/text/en.json")),
    ("ja", include_str!("../../resources/shellhook/text/ja.json")),
];

/// 一种语言的字。`{path}`、`{shell}`、`{program}` 印的时候换掉。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Words {
    pub installed: String,
    pub updated: String,
    pub foreign: String,
    pub removed: String,
    pub not_installed: String,
    pub old_hook: String,
    pub unknown_shell: String,
    pub status_installed: String,
    pub status_missing: String,
    pub status_foreign: String,
    pub status_stale: String,
    pub no_program: String,
    /// PowerShell 的执行策略不让加载启动文件（`{policy}`）。
    pub ps_policy: String,
    /// 这个终端还没和她说过话：斜杠命令没有会话可办。
    pub no_session: String,
    /// 闲置久了换了新会话（`{ago}` 是隔了多久）。
    pub renewed: String,
    /// 隔了多久的写法。
    pub units: Units,
    /// 斜杠命令在 shell 里不做（`{name}`）。
    pub slash_skipped: String,
    /// 斜杠命令在 shell 里还没做（`{name}`）。
    pub slash_pending: String,
    /// `/new` 开好了。
    pub slash_new: String,
    /// `/model` 换好了（`{name}`）。
    pub slash_model: String,
    /// `/theme` 换好了（`{name}`）。
    pub slash_theme: String,
    /// `/effort` 改好了（`{level}`）。
    pub slash_effort: String,
    /// `/restore` 恢复了。
    pub slash_restored: String,
    /// `/sessions` 换好了（`{title}`）。
    pub slash_sessions: String,
    /// `/persona`、`/preset` 写好了（`{name}`）。
    pub slash_persona: String,
    pub slash_preset: String,
    /// `doctor` 每一条前面的记号：好的、不好的。
    pub doctor_ok: String,
    pub doctor_bad: String,
    /// `doctor`：核心连得上、连不上（`{reason}`）。
    pub doctor_core_ok: String,
    pub doctor_core_bad: String,
    /// `doctor`：终端是不是真彩色。
    pub doctor_truecolor_ok: String,
    pub doctor_truecolor_bad: String,
    /// `doctor`：问不问得到终端的颜色。
    pub doctor_colors_ok: String,
    pub doctor_colors_bad: String,
    /// `doctor`：读剪贴板用哪个命令（`{tool}`）、一个都没有。
    pub doctor_clip_ok: String,
    pub doctor_clip_bad: String,
    /// 带的参数没写全、对得上几个（`{query}`）：后面一行一个列对得上的。
    pub slash_ambiguous: String,
    /// 带的参数什么都没对上（`{query}`）。
    pub slash_no_match: String,
}

/// 分钟、小时、天的写法，`{n}` 换成数。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Units {
    pub minutes: String,
    pub hours: String,
    pub days: String,
}

impl Words {
    /// 照环境认界面语言，取那一份。
    ///
    /// # Errors
    ///
    /// 语言表、这一份字读不懂（编进来的资源坏了）。
    pub fn detect() -> Result<Self, String> {
        let table = crate::language::LanguageTable::builtin()?;
        let language = table.detect(|name| std::env::var(name).ok());
        Self::of(language.code())
    }

    /// 这种语言的那一份；没编进来的照中文。
    ///
    /// # Errors
    ///
    /// 那一份读不懂。
    pub fn of(code: &str) -> Result<Self, String> {
        let (name, json) = TEXTS
            .iter()
            .find(|(c, _)| *c == code)
            .or_else(|| TEXTS.first())
            .ok_or("resources/shellhook/text/ 一份都没有")?;
        serde_json::from_str(json).map_err(|e| format!("resources/shellhook/text/{name}.json：{e}"))
    }
}

/// 把模板里的 `{名字}` 换成值。
pub fn fill(template: &str, values: &[(&str, &str)]) -> String {
    values
        .iter()
        .fold(template.to_string(), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), value)
        })
}
