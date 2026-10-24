//! `/context` 上下文框（蓝图 `tui.md`「上下文 `/context`」，2026-10-22 项目主人定）：打开时向核心要两样，`session.context`
//! （各块占多少，核心 CX-1 上）和 `usage.query` 按 `use` 分（这个会话连子代理的累计）；还没开会话的照选了的人格、预设要
//! 新会话的预览（核心 CX-1 下，参数同 `session.tools`），没有累计。回来了交给框画（`ui/context_box.rs`）。`↑` `↓` `PgUp`
//! `PgDn` 滚，`c` 照纯文字复制，`Esc` 关。
//!
//! 还没开会话时每选一样人格、预设也要一次预览，底栏照它写「用了多少 / 窗口（百分比）」（项目主人：「不然我都不知道预设
//! 消耗了多少」）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::json;

use super::{App, Panel};
use crate::core::{Command, Update};
use crate::oobe::asker::TAG_BASE;
use crate::ui::context_box::{ContextData, read_parts, read_uses};

/// 要组成的请求编号（`TAG_BASE - 17` 是 `tool_list.rs` 的）。
const PARTS_TAG: u64 = TAG_BASE - 18;
/// 要累计的请求编号。
const USES_TAG: u64 = TAG_BASE - 19;
/// 底栏要预览的请求编号。
const PREVIEW_TAG: u64 = TAG_BASE - 20;

impl App {
    /// 打开 `/context`：开着会话的问两样，没开的问预览。
    pub(super) fn open_context(&mut self) {
        let session = self.talk.transcript.session.clone();
        self.boxes.context = Some(ContextData::default());
        let params = match &session {
            Some(session) => json!({ "session": session }),
            None => self.new_session_params(),
        };
        self.link.core.send(Command::Ask {
            system: false,
            tag: PARTS_TAG,
            method: "session.context",
            params,
        });
        if let Some(session) = session {
            self.link.core.send(Command::Ask {
                system: false,
                tag: USES_TAG,
                method: "usage.query",
                params: json!({"session": session, "tree": true, "group": ["use"]}),
            });
        }
        self.boxes.panel = Some(Panel::Context { scroll: 0 });
    }

    /// 还没开会话、选了人格或者预设：要一次预览，底栏照它写。开着会话的不要。
    pub(super) fn preview_context(&mut self) {
        if self.talk.transcript.session.is_some() {
            return;
        }
        let params = self.new_session_params();
        self.link.core.send(Command::Ask {
            system: false,
            tag: PREVIEW_TAG,
            method: "session.context",
            params,
        });
    }

    /// 核心交回了一样：记下（框关了的不要）；回应归这里交回 `true`。
    pub(super) fn context_update(&mut self, update: &Update) -> bool {
        let Update::Answer { tag, result } = update else {
            return false;
        };
        if *tag == PREVIEW_TAG {
            // 拒了的（核心旧）不写；会话已经开了的不要。
            self.talk.preview = result
                .as_ref()
                .ok()
                .filter(|_| self.talk.transcript.session.is_none())
                .map(read_parts)
                .map(|shape| (shape.used(), shape.window));
            return true;
        }
        if *tag != PARTS_TAG && *tag != USES_TAG {
            return false;
        }
        let Some(data) = self.boxes.context.as_mut() else {
            return true;
        };
        let said = |r: &crate::core::Refusal| r.message.clone();
        if *tag == PARTS_TAG {
            data.parts = Some(result.as_ref().map(read_parts).map_err(said));
        } else {
            data.uses = Some(result.as_ref().map(read_uses).map_err(said));
        }
        true
    }

    /// 框开着时的按键。
    pub(super) fn context_key(&mut self, scroll: usize, key: KeyEvent) {
        let page = self.screen.panel_rows.len().max(1);
        let width = self.screen.areas.menu_text.width;
        let rows = crate::ui::context_box::count(&self.seen(), width);
        let last = rows.saturating_sub(page);
        let scroll = match key.code {
            KeyCode::Up => scroll.saturating_sub(1),
            KeyCode::Down => scroll + 1,
            KeyCode::PageUp => scroll.saturating_sub(page),
            KeyCode::PageDown => scroll + page,
            KeyCode::Char('c') => {
                let text = crate::ui::context_box::plain(&self.seen(), width);
                self.copy(&text);
                scroll
            }
            KeyCode::Esc => {
                self.boxes.panel = None;
                self.boxes.context = None;
                return;
            }
            _ => return,
        };
        self.boxes.panel = Some(Panel::Context {
            scroll: scroll.min(last),
        });
    }
}
