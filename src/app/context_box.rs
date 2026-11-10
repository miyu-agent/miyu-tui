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
/// 上下文上限换了以后重读限额的请求编号（核心 CTX-1）。
const CAP_TAG: u64 = TAG_BASE - 43;

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
            tag: PARTS_TAG,
            method: "session.context",
            params,
        });
        if let Some(session) = session {
            self.link.core.send(Command::Ask {
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
            tag: PREVIEW_TAG,
            method: "session.context",
            params,
        });
    }

    /// 正在看的会话换了上下文上限（`/context 128k`、im 网页的会话卡片，核心 CTX-1）：推来的只有上限，生效的窗口、压缩线
    /// 照 `session.context` 重读一次（设计 68 第四节，订阅回应里的限额当场换了，推送不带）。
    pub(super) fn context_cap_changed(&mut self, update: &Update) {
        if let Update::Push(crate::core::Push::ContextSet(_)) = update
            && let Some(session) = self.talk.transcript.session.clone()
        {
            self.link.core.send(Command::Ask {
                tag: CAP_TAG,
                method: "session.context",
                params: json!({ "session": session }),
            });
        }
    }

    /// 核心交回了一样：记下（框关了的不要）；回应归这里交回 `true`。
    pub(super) fn context_update(&mut self, update: &Update) -> bool {
        let Update::Answer { tag, result } = update else {
            return false;
        };
        if *tag == CAP_TAG {
            if let Ok(got) = result {
                let limits = &mut self.talk.transcript.limits;
                limits.window = got["window"].as_u64().filter(|w| *w > 0).or(limits.window);
                limits.compaction_line = got["line"].as_u64().or(limits.compaction_line);
                limits.context = got["context"].as_u64();
            }
            return true;
        }
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
        let scroll = match key.code {
            KeyCode::Up => scroll.saturating_sub(1),
            KeyCode::Down => scroll + 1,
            KeyCode::PageUp => scroll.saturating_sub(page),
            KeyCode::PageDown => scroll + page,
            // 回车：上下文各块的小项展开、收起（默认收着，2026-11-11 项目主人挑的样板）。
            KeyCode::Enter => {
                if let Some(data) = self.boxes.context.as_mut() {
                    data.open = !data.open;
                }
                scroll
            }
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
        // 展开、收起以后行数变了：照新的行数收住。
        let rows = crate::ui::context_box::count(&self.seen(), width);
        let last = rows.saturating_sub(page);
        self.boxes.panel = Some(Panel::Context {
            scroll: scroll.min(last),
        });
    }
}
