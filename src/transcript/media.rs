//! 她发出去的图（核心 X-19，蓝图「她发的图」）：一张一条，排在那一步工具后面；和回答一样收起前面那一段（那一步照旧在
//! 那一段里）。她给人看的文件（核心 D-5，蓝图「展示」）同样一条一条排。

use super::{Kind, Transcript};
use crate::core::Sent;

impl Transcript {
    /// 一步工具发出去了这几张图。
    pub(super) fn sent(&mut self, images: Vec<Sent>) {
        for image in images {
            self.finish_segment();
            self.push(Kind::Media, String::new());
            if let Some(last) = self.entries.last_mut() {
                last.media = Some(image);
            }
        }
    }

    /// 她给人看了一个文件（蓝图「展示」第 2 条）：排在那一步后面，同她发的图收起前面那一段。
    pub(super) fn shown(&mut self, file: crate::core::Shown) {
        self.finish_segment();
        self.push(Kind::Shown, String::new());
        if let Some(last) = self.entries.last_mut() {
            last.shown = Some(file);
        }
    }

    /// 后来同一个路径又展示过的那几条的编号：从新往旧看看得见的，路径见过的就是被换掉的（蓝图「展示」第 5 条）。撤掉了
    /// 新的那一轮，旧的跟着回来。
    pub fn replaced_shown(&self) -> std::collections::HashSet<u64> {
        let mut seen = std::collections::HashSet::new();
        self.entries
            .iter()
            .rev()
            .filter(|e| !e.hidden)
            .filter_map(|e| e.shown.as_ref().map(|f| (e.id, f.path.as_str())))
            .filter(|(_, path)| !seen.insert(*path))
            .map(|(id, _)| id)
            .collect()
    }
}
