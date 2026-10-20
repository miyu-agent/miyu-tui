//! `/status` 会话状态框（蓝图 `tui.md`「会话状态 `/status`」，2026-10-20 项目主人定）：和 `/usage` 一个位置、一个样子，
//! 左上角框线上写会话标题，一行一项。`↑` `↓` `PgUp` `PgDn` 滚，`c` 照纯文字复制整个框，`Esc` 关。画在 `ui/status_box.rs`。

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::{App, Panel};

impl App {
    /// 打开 `/status`。
    pub(super) fn open_status(&mut self) {
        self.boxes.panel = Some(Panel::Status { scroll: 0 });
    }

    /// 框开着时的按键。
    pub(super) fn status_key(&mut self, scroll: usize, key: KeyEvent) {
        let page = self.screen.panel_rows.len().max(1);
        let width = self.screen.areas.menu_text.width;
        let facts = crate::ui::status_box::facts(&self.seen());
        let rows = crate::ui::status_box::rows(&facts, &self.config, width).len();
        let last = rows.saturating_sub(page);
        let scroll = match key.code {
            KeyCode::Up => scroll.saturating_sub(1),
            KeyCode::Down => scroll + 1,
            KeyCode::PageUp => scroll.saturating_sub(page),
            KeyCode::PageDown => scroll + page,
            KeyCode::Char('c') => {
                let text = crate::ui::status_box::plain(&facts, &self.config, width);
                self.copy(&text);
                scroll
            }
            KeyCode::Esc => {
                self.boxes.panel = None;
                return;
            }
            _ => return,
        };
        self.boxes.panel = Some(Panel::Status {
            scroll: scroll.min(last),
        });
    }
}
