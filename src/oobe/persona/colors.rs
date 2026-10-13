//! 跟随头像的配色在引导里照谁取色（蓝图 `tui.md`「第一次打开的引导」第 23a 条、「主题」的「跟随头像的配色」）：人格那一步
//! 的这一位。

use std::path::PathBuf;

use super::PersonaStep;

/// 从哪取主色。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Colors {
    /// 人格写了主题色 `#rrggbb`。
    Seed(String),
    /// 这一次填了的头像文件。
    File(PathBuf),
    /// 人格已有的头像：编号、版本（照版本要来存下的那一份）。
    Avatar {
        /// 人格的编号。
        persona: String,
        /// 头像的版本。
        version: String,
    },
}

impl PersonaStep {
    /// 写了主题色的照它（同核心：主题色压过头像）；这一次填了头像文件的照文件（文件不在的当没有）；已有的人格有头像的照
    /// 它的头像；都没有的是 `None`。
    pub fn colors(&self) -> Option<Colors> {
        let existing = self.existing.as_ref();
        if let Some(seed) = existing.and_then(|e| e.seed.clone()) {
            return Some(Colors::Seed(seed));
        }
        if let Some(path) = self.avatar_path() {
            return path.is_file().then_some(Colors::File(path));
        }
        let existing = existing?;
        Some(Colors::Avatar {
            persona: existing.id.clone(),
            version: existing.avatar.clone()?,
        })
    }
}
