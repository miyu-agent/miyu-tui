//! 新会话开头那个换掉输入框的人格、预设框开着（列表还在读也算）时打的字、粘贴的字（蓝图 `tui.md`「新会话：人格、工作区」
//! 第 1c 条，2026-10-26 项目主人定）：照样进输入框，框不关；回车照旧选框里的，框关了字还在，再回车发出去。原来这些字
//! 被框吞掉，回车又选了预设，刚起来马上发的第一句就没了（项目主人报「刚启动后马上发的第一条长消息有时会被吞掉」）。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::App;

impl App {
    /// 框开着时的一个键：是打的字的进输入框，交回 `true`（框不再管它）。还没打字时 `e`、`j`、`k` 归框（编辑、上下），
    /// 打了字以后都当字；退格、删、左右有字了才归输入框。方向键、回车、`Esc` 照旧归框。
    pub(super) fn start_box_typed(&mut self, key: KeyEvent) -> bool {
        if !self.start_box_shown()
            || key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return false;
        }
        let typed = !self.typing.input.editor.is_empty();
        let ours = match key.code {
            KeyCode::Char(c) => typed || !matches!(c, 'e' | 'E' | 'j' | 'k'),
            KeyCode::Backspace | KeyCode::Delete | KeyCode::Left | KeyCode::Right => typed,
            _ => false,
        };
        if ours {
            self.typing.input.key(key);
        }
        ours
    }
}
