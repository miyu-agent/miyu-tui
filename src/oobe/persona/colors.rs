//! 跟随头像的配色在引导里照谁取色（蓝图 `tui.md`「第一次打开的引导」第 23a 条、「主题」的「跟随头像的配色」）：人格那一步
//! 的这一位。

use std::path::PathBuf;

use super::PersonaStep;

/// 从哪取主色。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Colors {
    /// 存好的人格，核心给的（2026-10-14 项目主人定：头像取色归核心）：自选的主题色（没选的是 `None`）、头像的候选主色
    /// `#rrggbb`（好的在前）。
    Given {
        /// 自选的主题色。
        seed: Option<String>,
        /// 头像的候选。
        seeds: Vec<String>,
    },
    /// 这一次填了、人格还没存的头像文件：照核心的门面现取（同一份代码）。
    File(PathBuf),
}

impl PersonaStep {
    /// 写了主题色的照它（同核心：主题色压过头像）；这一次填了头像文件的照文件（文件不在的当没有）；已有的人格有头像候选的
    /// 照核心给的；都没有的是 `None`。
    pub fn colors(&self) -> Option<Colors> {
        let existing = self.existing.as_ref();
        let given = |seed: Option<String>| Colors::Given {
            seed,
            seeds: existing.and_then(|e| e.seeds.clone()).unwrap_or_default(),
        };
        if let Some(seed) = existing.and_then(|e| e.seed.clone()) {
            return Some(given(Some(seed)));
        }
        if let Some(path) = self.avatar_path() {
            return path.is_file().then_some(Colors::File(path));
        }
        existing?.seeds.as_ref().filter(|s| !s.is_empty())?;
        Some(given(None))
    }
}
