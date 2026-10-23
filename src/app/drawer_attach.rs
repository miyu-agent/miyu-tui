//! 抽屉里贴图（蓝图「确认和提问的抽屉」第 3a 条，核心 Q-A；2026-10-22 项目主人：「问问题、写理由时用户需要图片辅助表达」）：
//! `Ctrl+V` 时剪贴板里复制的图片文件、截图，收成这一道（确认是「不允许」）的图，字下面一行小块，块上的字照输入框的编号
//! 接着编。现在的模型不收图的，同输入框把路径写进字里。

use super::App;
use super::paste::{IMAGE, attach_rule};
use crate::clipboard;
use crate::drawer::Pasted;
use crate::input::{Dropped, dropped, quoted};

impl App {
    /// 剪贴板里有图就收进开着的抽屉，交回 `true`；没有图的交回 `false`，照字粘。
    pub(super) fn drawer_attach(&mut self) -> bool {
        let files = self.clipboard_images();
        if files.is_empty() {
            return false;
        }
        // 模型不收图：路径写进字里，提示一句（同输入框「照模型收」）。
        if self.refused_kinds().iter().any(|k| k == IMAGE) {
            let text: Vec<String> = files.iter().map(|f| quoted(f)).collect();
            self.drawer_paste(&text.join(" "));
            self.tell_demoted(&[IMAGE.to_string()]);
            return true;
        }
        let rule = attach_rule(&self.config);
        let base = self
            .talk
            .transcript
            .attachment_counts()
            .get(IMAGE)
            .copied()
            .unwrap_or(0);
        let Some(drawer) = self.talk.drawers.current.as_mut() else {
            return false;
        };
        let mut taken = true;
        for file in files {
            let label = rule.label(IMAGE, base + drawer.pasted_count() + 1);
            taken &= drawer.attach(Pasted { label, file });
        }
        if !taken {
            let note = self.config.text.attach_deny_only.clone();
            self.hint(note, false);
        }
        true
    }

    /// 剪贴板里的图：复制的图片文件（`text/uri-list`），没有的读截图（落进暂存目录）。
    fn clipboard_images(&self) -> Vec<std::path::PathBuf> {
        let home = std::env::var("HOME").ok();
        let rule = attach_rule(&self.config);
        let copied: Vec<_> = clipboard::read_files()
            .and_then(|list| dropped(&list, home.as_deref(), &rule))
            .into_iter()
            .flatten()
            .filter_map(|item| match item {
                Dropped::File(path, kind) if kind == IMAGE => Some(path),
                _ => None,
            })
            .collect();
        if !copied.is_empty() {
            return copied;
        }
        let dir = self.typing.staging.as_ref().map(clipboard::Staging::dir);
        dir.and_then(clipboard::read_image).into_iter().collect()
    }
}
