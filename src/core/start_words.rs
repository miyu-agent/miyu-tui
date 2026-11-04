//! 核心起不来、认得出是哪一种的，照界面的语言说（排期第 43 条，核心 LC-1；蓝图「连核心」第 8 条）：这时问不到
//! `human.get`，照核心程序旁边的资源目录自己读 `core/start/<代码>` 那一句。

use std::path::{Path, PathBuf};

use miyu_client::places::{Env, ResourceRoot};
use miyu_client::protocol::Said;
use miyu_client::texts::Human;

/// 核心起不来，带着代码（`data_root_not_ours` 这些）：界面照自己的语言换成一句话（[`StartRefusal::say`]）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartRefusal {
    /// 哪一种。
    pub code: String,
    /// 核心的原话：读不到那一句的照它。
    pub reason: String,
    /// 数据根，填 `{path}`。
    pub root: PathBuf,
    /// 核心程序（`MIYU_CORE_BIN`），资源目录在它旁边；没给的是 `None`。
    pub bin: Option<PathBuf>,
}

impl StartRefusal {
    /// 照 `language` 说：资源目录里有这一句的照它，没有的照原话。
    pub fn say(&self, language: &str) -> String {
        self.resources()
            .and_then(|resources| self.words(&resources, language))
            .unwrap_or_else(|| self.reason.clone())
    }

    /// 核心的资源目录：照核心程序找（`MIYU_RESOURCES` 设了的照它）。
    fn resources(&self) -> Option<ResourceRoot> {
        let mut env = Env::current();
        if let Some(bin) = self.bin.as_deref().and_then(program) {
            env.exe = Some(bin.canonicalize().unwrap_or(bin));
        }
        ResourceRoot::locate(&env).ok()
    }

    /// `resources` 里 `core/start/<代码>` 那一句；读不到、没有的是 `None`。
    fn words(&self, resources: &ResourceRoot, language: &str) -> Option<String> {
        let human = Human::load(resources, language).ok()?;
        let mut said = Said::new(format!("core/start/{}", self.code));
        said.fields
            .insert("path".into(), self.root.display().to_string());
        human.say(&said)
    }
}

/// 程序在哪：带路径的照它，光名字的照 `PATH` 找。
fn program(bin: &Path) -> Option<PathBuf> {
    if bin.components().count() > 1 {
        return Some(bin.to_path_buf());
    }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(bin))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests;
