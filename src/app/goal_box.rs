//! 长期目标的浮窗（蓝图 `tui.md`「长期目标」第 5a 条，2026-10-26 项目主人定）：点目标那一行开，和 `/status` 一个位置、
//! 一个样子；`↑` `↓` `PgUp` `PgDn` 滚，`Esc` 关。画在 `ui/goal_box.rs`。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Position;
use unicode_width::UnicodeWidthStr;

use super::{App, Panel};

impl App {
    /// 点在目标那一行写着的字上（右对齐的那一截）：交回 `true`，浮窗开了。
    pub(super) fn goal_clicked(&mut self, at: Position) -> bool {
        let area = self.screen.areas.goal;
        if at.y != area.y || !area.contains(at) || self.talk.viewing_title().is_some() {
            return false;
        }
        let now = jiff::Timestamp::now();
        let Some(text) =
            crate::ui::goal_tag::goal(self.talk.transcript.goal.as_ref(), &self.config, now)
        else {
            return false;
        };
        let width = u16::try_from(text.width()).unwrap_or(u16::MAX);
        if at.x < area.right().saturating_sub(width) {
            return false;
        }
        self.boxes.panel = Some(Panel::Goal { scroll: 0 });
        true
    }

    /// 浮窗开着时的按键。
    pub(super) fn goal_key(&mut self, scroll: usize, key: KeyEvent) {
        let page = self.screen.panel_rows.len().max(1);
        let scroll = match key.code {
            KeyCode::Up => scroll.saturating_sub(1),
            KeyCode::Down => scroll + 1,
            KeyCode::PageUp => scroll.saturating_sub(page),
            KeyCode::PageDown => scroll + page,
            KeyCode::Esc => {
                self.boxes.panel = None;
                return;
            }
            _ => return,
        };
        let width = self.screen.areas.menu_text.width;
        let rows: usize = self.talk.transcript.goal.as_ref().map_or(0, |g| {
            let now = (width, jiff::Timestamp::now());
            crate::ui::goal_box::rows(g, None, &self.config, now).len()
        });
        self.boxes.panel = Some(Panel::Goal {
            scroll: scroll.min(rows.saturating_sub(page)),
        });
    }
}
