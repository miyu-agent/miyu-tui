//! `/tool-list` 工具框（蓝图 `tui.md`「工具 `/tool-list`」，2026-10-22 项目主人定）：打开时向核心要 `session.tools`（核心
//! S-T）：开着会话的问这个会话；还没开会话的照这个新会话选了的人格、预设问，还没选的不写、核心照默认的（同 `session.create`）。
//! 回来了交给框画（`ui/tool_list.rs`）。`↑` `↓` `PgUp` `PgDn` 滚，`c` 照纯文字复制，`Esc` 关。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::{App, Panel};
use crate::core::{Command, Update};
use crate::oobe::asker::TAG_BASE;

/// 要工具列表的请求编号（`TAG_BASE - 16` 是 `head_settings.rs` 读「不问人格预设」的）。
const TOOLS_TAG: u64 = TAG_BASE - 17;

impl App {
    /// 打开 `/tool-list`：先清掉上一次读回来的，问一次。
    pub(super) fn open_tool_list(&mut self) {
        self.boxes.tools = None;
        let params = match &self.talk.transcript.session {
            Some(session) => json!({ "session": session }),
            None => self.new_session_params(),
        };
        self.link.core.send(Command::Ask {
            tag: TOOLS_TAG,
            method: "session.tools",
            params,
        });
        self.boxes.panel = Some(Panel::Tools { scroll: 0 });
    }

    /// 还没开会话：照选了的人格、预设问（无人格、无工具是 `null`），还没选的不写（`session.tools`、`session.context` 都用）。
    pub(super) fn new_session_params(&self) -> Value {
        let mut params = json!({});
        if let Some(persona) = &self.talk.persona.chosen {
            params["persona"] = if persona.is_empty() {
                Value::Null
            } else {
                json!(persona)
            };
        }
        if let Some(preset) = &self.talk.preset.chosen {
            params["preset"] = crate::core::preset_param(preset);
        }
        params
    }

    /// 核心交回了工具列表：记下（框关了的不要）；回应归这里交回 `true`。
    pub(super) fn tool_list_update(&mut self, update: &Update) -> bool {
        let Update::Answer { tag, result } = update else {
            return false;
        };
        if *tag != TOOLS_TAG {
            return false;
        }
        if matches!(self.boxes.panel, Some(Panel::Tools { .. })) {
            self.boxes.tools = Some(match result {
                Ok(got) => Ok(crate::ui::tool_list::read(got)),
                Err(refusal) => Err(refusal.message.clone()),
            });
        }
        true
    }

    /// 框开着时的按键。
    pub(super) fn tool_list_key(&mut self, scroll: usize, key: KeyEvent) {
        let page = self.screen.panel_rows.len().max(1);
        let width = self.screen.areas.menu_text.width;
        let rows = crate::ui::tool_list::count(&self.seen(), width);
        let last = rows.saturating_sub(page);
        let scroll = match key.code {
            KeyCode::Up => scroll.saturating_sub(1),
            KeyCode::Down => scroll + 1,
            KeyCode::PageUp => scroll.saturating_sub(page),
            KeyCode::PageDown => scroll + page,
            KeyCode::Char('c') => {
                let text = crate::ui::tool_list::plain(&self.seen(), width);
                self.copy(&text);
                scroll
            }
            KeyCode::Esc => {
                self.boxes.panel = None;
                self.boxes.tools = None;
                return;
            }
            _ => return,
        };
        self.boxes.panel = Some(Panel::Tools {
            scroll: scroll.min(last),
        });
    }
}

#[cfg(test)]
mod tests;
