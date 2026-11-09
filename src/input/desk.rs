//! 输入框里跟着会话走的那一份（蓝图 `tui.md`「后台命令、子代理和侧边栏」第 7 条最近会话的子条，发版前 check 第 17 条）：
//! 没发的字之外，`Ctrl+C` 清掉的（空着按 `↑` 找回的）、`Ctrl+S` 暂存的、在编辑的上一句、跑命令的模式、翻输入历史翻到哪，
//! 都是这个会话的，切走时一起交出去，切回来一起放回。输入历史本身是大家的，不在里面。

use super::{Draft, InputBox};

/// 一个会话的输入框。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Desk {
    /// 框里没发的字，连同块。
    typed: Draft,
    /// `Ctrl+C` 清掉的那一句。
    cleared: Option<Draft>,
    /// `Ctrl+S` 暂存的。
    stash: Option<Draft>,
    /// 在编辑上一句：改之前的那句。
    editing: Option<Draft>,
    /// 在跑命令的模式里。
    shell: bool,
    /// 撤销时放回来的那句。
    put_back: Option<String>,
    /// 翻输入历史翻到第几条、照什么开头筛、翻之前没发的那句。
    browsing: Option<usize>,
    prefix: String,
    before: Draft,
}

impl Desk {
    /// 什么都没有：不用记。
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

impl InputBox {
    /// 切走：交出这个会话的那一份，框回到空的。
    pub fn take_desk(&mut self) -> Desk {
        self.goal_col = None;
        self.scroll = 0;
        self.dragging = false;
        Desk {
            typed: self.editor.take_draft(),
            cleared: self.cleared.take(),
            stash: self.stash.take(),
            editing: self.editing.take(),
            shell: std::mem::take(&mut self.shell),
            put_back: self.put_back.take(),
            browsing: self.browsing.take(),
            prefix: std::mem::take(&mut self.prefix),
            before: std::mem::take(&mut self.draft),
        }
    }

    /// 切过来：放回这个会话的那一份（没有的是空的），光标在字的末尾。
    pub fn put_desk(&mut self, desk: Desk) {
        self.editor.set_draft(desk.typed);
        self.cleared = desk.cleared;
        self.stash = desk.stash;
        self.editing = desk.editing;
        self.shell = desk.shell;
        self.put_back = desk.put_back;
        self.browsing = desk.browsing.filter(|at| *at <= self.history.len());
        self.prefix = desk.prefix;
        self.draft = desk.before;
        self.follow = true;
    }
}
