//! `/status` 会话状态框（蓝图 `tui.md`「会话状态 `/status`」，2026-10-20 项目主人定）：和 `/usage` 一个位置、一个样子，
//! 左上角框线上写会话标题，一行一项。`↑` `↓` `PgUp` `PgDn` 滚，`c` 照纯文字复制整个框，`Esc` 关。画在 `ui/status_box.rs`。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::json;

use super::{App, Panel};
use crate::core::{Command, Update};
use crate::oobe::asker::TAG_BASE;

/// 问这个会话每个模型用了多少的那一次。
const MODELS_TAG: u64 = TAG_BASE - 22;

impl App {
    /// 打开 `/status`；开着的会话另问一次每个模型用了多少（`usage.query` 照模型分，连子代理），回来了照它写「各模型」。
    pub(super) fn open_status(&mut self) {
        self.boxes.status_models.clear();
        if let Some(session) = self.talk.transcript.session.clone() {
            self.link.core.send(Command::Ask {
                tag: MODELS_TAG,
                method: "usage.query",
                params: json!({"session": session, "tree": true, "group": ["model"]}),
            });
        }
        self.boxes.panel = Some(Panel::Status { scroll: 0 });
    }

    /// 每个模型的用量回来了：记下，框照它画（框关了的也记着，下次开时先清掉）；回应归这里交回 `true`。
    pub(super) fn status_update(&mut self, update: &Update) -> bool {
        let Update::Answer { tag, result } = update else {
            return false;
        };
        if *tag != MODELS_TAG {
            return false;
        }
        if let Ok(result) = result {
            self.boxes.status_models = crate::core::usage_rows(result);
        }
        true
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
