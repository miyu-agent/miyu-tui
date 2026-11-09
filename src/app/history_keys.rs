//! 输入历史列表、`/jump` 开着时的按键（蓝图 `tui.md`「输入历史列表」第 2–4 条，「跳到说过的话」第 3 条）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::App;

impl App {
    /// 输入历史列表开着时的按键：`↑`、`Ctrl+R` 往更早的走，`↓` 往更新的走，打字进搜索，
    /// `Enter` 挑中放进输入框，`Esc` 关掉（`tui.md`「输入历史列表」第 2–4 条；`Tab` 不做什么，第 7 条）。按键模式是 Vim 的
    /// 搜索照会话列表那一套：`/` 进搜索，平时 `j` `k` 上下（发版前 check 第 31 条，`list_search.rs`）。
    pub(super) fn history_key(&mut self, key: KeyEvent) {
        use crate::list_search::{self, Step};
        use ratatui::crossterm::event::KeyModifiers;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if !ctrl && !key.modifiers.contains(KeyModifiers::ALT) {
            let history = &mut self.typing.history;
            match list_search::key(
                self.config.layout.vim,
                &mut history.typing,
                &history.query,
                key,
            ) {
                Step::Query(query) => return history.set_query(query),
                Step::Kept => return,
                Step::Pass => {}
            }
        }
        let count = self.typing.history.matches(&self.list_items().0).len();
        match key.code {
            KeyCode::Up => self.typing.history.older(count),
            KeyCode::Char('r') if ctrl => self.typing.history.older(count),
            KeyCode::Down => self.typing.history.newer(),
            KeyCode::Enter => self.pick_history(),
            KeyCode::Esc => self.typing.history.close(),
            _ => {}
        }
    }

    /// 挑中选中的那一条：放进输入框，关掉列表；`/jump` 的跳过去（「跳到说过的话」第 3 条）。
    pub(super) fn pick_history(&mut self) {
        let (text, id) = {
            let (list, ids) = self.list_items();
            let found = self.typing.history.matches(&list);
            let picked = found.get(self.typing.history.selected).copied();
            // 列表是照先后排的一份，挑中的是其中哪一句：照位置找回它的条目编号。
            let id = picked
                .and_then(|p| list.iter().position(|s| std::ptr::eq(s, p)))
                .and_then(|i| ids.get(i).copied());
            (picked.map(|sent| sent.draft.text.clone()), id)
        };
        let jump = self.typing.history.jump;
        self.typing.history.close();
        match (jump, id, text) {
            (true, Some(id), _) => self.jump_to(id),
            (false, _, Some(text)) => self.typing.input.pick(&text),
            _ => {}
        }
    }
}
