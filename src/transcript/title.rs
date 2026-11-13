//! 会话标题（`session.meta_changed`）：输入框上边框右边只写人要的标题（蓝图「输入框」第 14 条）。

use super::Transcript;

impl Transcript {
    /// 推来新标题：去掉标题推来的是空的，读成没有标题。人起的（`by` 是人）照人起的；不带字的 `/rename` 重起的，核心 TT-2
    /// 起推来时 `by` 就是要它的人、会话列表里 `named` 为真（2026-11-13 主人 1113 初验第 17 条：`/rename` 以后框线上没有新标题；
    /// 原来终端自己记一下「刚要过」顶着，重开会话就丢，核心改了以后去掉）。
    pub(super) fn titled(&mut self, title: String, named: bool) {
        self.named = named && !title.is_empty();
        self.title = (!title.is_empty()).then_some(title);
    }
}
