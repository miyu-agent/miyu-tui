//! 会话标题（`session.meta_changed`）：输入框上边框右边只写人要的标题（蓝图「输入框」第 14 条）。

use super::Transcript;

impl Transcript {
    /// 推来新标题：去掉标题推来的是空的，读成没有标题。人起的（`by` 是人）照人起的；人打了不带字的 `/rename`、核心收下了
    /// 以后内核推来的那一个也算人要的（2026-11-13 主人 1113 初验第 17 条：`/rename` 以后框线上没有新标题）。
    pub(super) fn titled(&mut self, title: String, named: bool) {
        let asked = std::mem::take(&mut self.retitle_asked);
        self.named = (named || asked) && !title.is_empty();
        self.title = (!title.is_empty()).then_some(title);
    }
}
