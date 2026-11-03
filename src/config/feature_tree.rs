//! 功能树的字（`text/zh.json` 的 `feature_tree`；蓝图 `tui.md`「配置页」第 38 条「功能树」）。

use std::collections::HashMap;

use serde::Deserialize;

/// 功能树的字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureTreeTexts {
    /// 一组什么都没有时写的，照组的 `kind`（`skills`、`scripts`、`mcp`）；别的种类照 `other`。
    pub empty: HashMap<String, String>,
    /// 写了但这台机器没装的功能，名字后面黄字。
    pub missing: String,
}
