//! 她发出去的图（核心 X-19，蓝图「她发的图」）：一张一条，排在那一步工具后面；和回答一样收起前面那一段（那一步照旧在
//! 那一段里）。

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
}
