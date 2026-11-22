//! mermaid 图的收发（蓝图 `tui.md`「图片、公式和 mermaid 图」第 4 条）：排正文时记进单子的源码，主循环每一帧以后交给
//! 核心画；回来了记进账，排的时候问过账的条目下一帧重排（占位换成图），别的条目用记着的行（施工 LW-24）。

use super::App;
use crate::core::{Command, Update};

impl App {
    /// 把单子上的发出去（`tick`）。
    pub(super) fn send_diagram_asks(&mut self) {
        for source in self.media.diagrams.borrow_mut().take() {
            self.link.core.send(Command::RenderMermaid(source));
        }
    }

    /// 单子上有没发的：主循环马上醒来发（`deadline.rs`）。
    pub(super) fn diagram_asks_pending(&self) -> bool {
        self.media.diagrams.borrow().pending()
    }

    /// 图回来了：记进账（用到账的条目下一帧重排，`ui/row_cache`，施工 LW-24），交回 `true`。核心断开了没回来的在 `lost.rs` 一起办。
    pub(super) fn diagram_update(&mut self, update: &mut Option<Update>) -> bool {
        match update.take() {
            Some(Update::Mermaid { source, svg }) => {
                self.media.diagrams.borrow_mut().got(source, svg);
                true
            }
            other => {
                *update = other;
                false
            }
        }
    }
}
