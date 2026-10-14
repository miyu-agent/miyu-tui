//! 开着的框和整屏页（蓝图 `app.md` 的 `boxes` 块）：后台面板、会话列表、`/model`、`/effort`、`/usage` 的框，配置页、
//! 第一次打开的引导，和它们交给编辑器写的那份、引导走没走过的标记。

/// 开着的框和整屏页。
#[derive(Default)]
pub struct Boxes {
    /// 开着的面板（后台）。
    pub panel: Option<super::Panel>,
    /// `/sessions` 开着时的会话列表（`switch.rs`）。
    pub session_list: Option<crate::session_list::SessionList>,
    /// `/model` 开着时的框（`models.rs`）。
    pub model_list: Option<crate::model_list::ModelList>,
    /// `/effort` 框里那个模型的几级；还没交回来的是 `None`（`effort.rs`）。
    pub efforts: Option<crate::core::Efforts>,
    /// `/usage` 框读回来的（开着才有）。
    pub usage: Option<super::usage::UsageData>,
    /// 开着的配置页（`/config`、`--page config`，蓝图「配置页」）：开着时整屏归它。
    pub settings: Option<crate::settings::Settings>,
    /// 配置页交给编辑器的提示词写在哪个临时文件（`settings.rs`）。
    pub settings_prompt: Option<std::path::PathBuf>,
    /// 第一次打开的引导；没开是 `None`（`oobe.rs`）。
    pub oobe: Option<crate::oobe::Oobe>,
    /// 引导交给编辑器的字写在哪个临时文件（`oobe.rs`）。
    pub oobe_editing: Option<std::path::PathBuf>,
    /// 读过 `ui.welcomed` 了：一次启动只读一次（「第一次打开的引导」第 1 条）。
    pub welcome_checked: bool,
    /// `ui.welcomed` 问到了（或者连不上核心）：这以前不画首页（[`super::seen::Seen::welcome_pending`]）。
    pub welcome_known: bool,
    /// 人格列表到了（或者连不上核心）：首页的头像、跟随头像的主色照它，这以前也不画首页，免得先闪一下吉祥物和 tokyonight
    /// （2026-10-15 冷启动查出）。
    pub personas_known: bool,
}
