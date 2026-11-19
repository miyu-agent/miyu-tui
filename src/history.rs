//! 输入历史列表的状态（蓝图 `tui.md`「输入历史列表」）：开没开、搜索的字、选中哪一条。
//!
//! 列的是输入框记着的发过的话和命令（和 `↑`、`↓` 翻的是同一份）。对得上的照「最新的在前」排，
//! 选中的是其中第几条：0 是最新的，画的时候最新的贴着输入框。`/jump`（「跳到说过的话」）用同一个列表，列的是这个会话
//! 正文里你说的话，同样的字说了两次是两条。

use crate::input::Sent;

/// 列表的状态。
#[derive(Debug, Default)]
pub struct History {
    /// 开着。
    pub open: bool,
    /// 开的是 `/jump`：列这个会话里你说的话，`Enter` 跳过去（不是填进输入框）。
    pub jump: bool,
    /// 框里最下面搜索那一行打的字（「输入历史列表」第 1、3 条）。
    pub query: String,
    /// 选中的是对得上的第几条，0 是最新的。
    pub selected: usize,
    /// 鼠标钉住的露出来的那一段（`menu::pin`）；按键选、打字放开。
    pub pinned: Option<usize>,
    /// 按键模式是 Vim 时按了 `/` 正在打搜索的字（`list_search.rs`，发版前 check 第 31 条）；没在打时 `j` `k` 上下。
    pub typing: bool,
    /// `/jump` 开着、更早的页还没读上来时按到头以后多按的 `↑` 几下（「跳到说过的话」第 2 条）：页读上来接着往上走。
    pub owed: usize,
}

impl History {
    /// 打开：搜索的字清空，选中最新的一条。
    pub fn open(&mut self) {
        *self = Self {
            open: true,
            ..Self::default()
        };
    }

    /// `/jump` 打开：同上，列的是这个会话里你说的话。
    pub fn open_jump(&mut self) {
        *self = Self {
            open: true,
            jump: true,
            ..Self::default()
        };
    }

    /// 关掉。
    pub fn close(&mut self) {
        self.open = false;
    }

    /// 对得上的几条：模糊搜索（`fuzzy.rs`），越像的越前，一样像的最新的在前；没打字的最新的在前。一样的字只留最新的那一次
    /// （`/jump` 的每条都留：位置不一样）。
    pub fn matches<'a>(&self, sent: &'a [Sent]) -> Vec<&'a Sent> {
        let mut seen: Vec<&Sent> = Vec::new();
        for one in sent.iter().rev() {
            let text = &one.draft.text;
            let again = !self.jump && seen.iter().any(|s| s.draft.text == *text);
            if !again {
                seen.push(one);
            }
        }
        crate::fuzzy::rank(&self.query, seen, |s| vec![s.draft.text.as_str()])
    }

    /// 往更早的走（`↑`、再按 `Ctrl+R`），到头就停。`count` 是对得上的条数；`more` 是更早的还有、没读上来：到头以后多按的
    /// 记着（[`History::settle`]）。
    pub fn older(&mut self, count: usize, more: bool) {
        self.pinned = None;
        if self.selected + 1 < count {
            self.selected += 1;
        } else if more {
            self.owed += 1;
        }
    }

    /// 往更新的走（`↓`），到头就停；记着没走的 `↑` 先抵掉一下。
    pub fn newer(&mut self) {
        self.pinned = None;
        if self.owed > 0 {
            self.owed -= 1;
        } else {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    /// 更早的一页读上来了，对得上的有 `count` 条：记着的 `↑` 接着走，走到头还欠的等下一页；读完了（`more` 是 `false`）
    /// 不再欠。
    pub fn settle(&mut self, count: usize, more: bool) {
        let room = count.saturating_sub(self.selected + 1);
        let step = self.owed.min(room);
        self.selected += step;
        self.owed = if more { self.owed - step } else { 0 };
    }

    /// 搜索的字后面接上 `text`，选中回到最新的一条。
    pub fn type_text(&mut self, text: &str) {
        self.query.push_str(text);
        self.selected = 0;
        self.owed = 0;
        self.pinned = None;
    }

    /// 搜索的字换成 `query`（`list_search.rs` 交回的），选中回到最新的一条。
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.selected = 0;
        self.owed = 0;
        self.pinned = None;
    }
}

#[cfg(test)]
mod tests {
    use super::History;
    use crate::input::{Draft, Sent};

    fn texts(found: Vec<&Sent>) -> Vec<&str> {
        found.iter().map(|s| s.draft.text.as_str()).collect()
    }

    fn sent() -> Vec<Sent> {
        [
            "跑一下全部测试",
            "看看 README",
            "把失败的测试修掉",
            "看看 README",
        ]
        .map(|text| Sent {
            draft: Draft::plain(text),
            at: std::time::SystemTime::now(),
        })
        .to_vec()
    }

    #[test]
    fn newest_first_and_each_text_once() {
        let mut h = History::default();
        h.open();
        assert_eq!(
            texts(h.matches(&sent())),
            vec!["看看 README", "把失败的测试修掉", "跑一下全部测试"]
        );
    }

    #[test]
    fn the_jump_list_keeps_each_time_the_same_words_were_said() {
        let mut h = History::default();
        h.open_jump();
        assert_eq!(
            texts(h.matches(&sent())),
            vec![
                "看看 README",
                "把失败的测试修掉",
                "看看 README",
                "跑一下全部测试"
            ],
            "说了两次是两个位置"
        );
    }

    #[test]
    fn typing_filters_ignoring_case_and_goes_back_to_the_newest() {
        let mut h = History::default();
        h.open();
        h.older(3, false);
        h.type_text("readme");
        assert_eq!(
            (texts(h.matches(&sent())), h.selected),
            (vec!["看看 README"], 0)
        );
        h.set_query(String::new());
        h.type_text("测试");
        assert_eq!(
            texts(h.matches(&sent())),
            vec!["把失败的测试修掉", "跑一下全部测试"]
        );
    }

    #[test]
    fn moving_stops_at_both_ends() {
        let mut h = History::default();
        h.open();
        h.newer();
        assert_eq!(h.selected, 0);
        for _ in 0..5 {
            h.older(3, false);
        }
        assert_eq!(h.selected, 2);
    }
}
