//! 编辑窗里哪几行露着：池的窗按 `/` 筛成员（2026-10-29 项目主人：模型一多找不到，同加供应商的目录窗那样能搜），筛的字照
//! 模糊匹配认引用，对不上的成员行不画、上下挪跳过，勾过的照旧算数，清掉筛的字就都回来；只填 key 的窗只露 key 那一行。

use super::{Field, Focus, Form, Val};
use crate::input::Editor;

impl Form {
    /// 换成只填 key 的窗（`name` 是目录里的名字）：光标停在 key 那一行、直接在打。
    pub fn key_only(mut self, name: String) -> Self {
        self.key_only = Some(name);
        if let Some(at) = self.rows.iter().position(|r| r.field == Field::Key) {
            self.focus = Focus::Row(at);
            self.editing = Some(Editor::default());
        }
        self
    }

    /// 有成员可筛（池的窗）。
    pub fn searchable(&self) -> bool {
        self.rows
            .iter()
            .any(|r| matches!(r.val, Val::Member { .. }))
    }

    /// 第 `i` 行露不露：只填 key 的窗只露 key；成员行要对得上筛的字，别的行都露。
    pub fn shows(&self, i: usize) -> bool {
        if self.key_only.is_some() {
            return self.rows[i].field == Field::Key;
        }
        match &self.rows[i].val {
            Val::Member { reference, .. } => {
                crate::fuzzy::hit(&self.search.0, [reference.as_str()])
            }
            _ => true,
        }
    }

    /// 打了筛的字：光标挪到第一行对得上的成员（同目录窗，打完直接能勾）；一个都对不上的停在按钮上。
    pub fn focus_first_match(&mut self) {
        let member = (0..self.rows.len())
            .find(|&i| matches!(self.rows[i].val, Val::Member { .. }) && self.shows(i));
        self.focus = member.map_or(Focus::Button(0), Focus::Row);
    }

    /// 筛的字清掉了、上下挪了：光标停着的行还露着就不动，藏起来了的照 [`Form::focus_first_match`]。
    pub fn refocus(&mut self) {
        if let Focus::Row(i) = self.focus
            && !self.shows(i)
        {
            self.focus_first_match();
        }
    }
}
