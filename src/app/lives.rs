//! 快速跳转人生会话（蓝图 `tui.md`「推演人生」第 1 条；2026-11-17 项目主人本机验收第 29 条，核心 LF-12）：光打 `/life` 的回应
//! 带所有人格没结束的人生会话（`lives`）：一条直接跳过去，几条在输入框上面开一个列表挑（人格的名字、在跑还是暂停、下次
//! 几点），`Enter` 跳过去。还没开会话（首页、`/new` 以后）时不为它开会话：直接 `life.list`，照同一个规矩办。别的 `/life …`
//! 照旧交给核心，回执照它写。画在 `ui/lives.rs`。

use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::Position;
use serde_json::json;

use super::{App, Panel};
use crate::core::{Command, Life, Update};
use crate::oobe::asker::TAG_BASE;

/// 还没开会话时光打 `/life` 的 `life.list` 请求编号。
const LIST_TAG: u64 = TAG_BASE - 51;

/// 这一行是光打的 `/life`（前后的空白不算）。
pub(super) fn bare(line: &str) -> bool {
    line.trim() == "/life"
}

impl App {
    /// 斜杠命令办了：光打 `/life` 带回的人生会话一条的跳过去、几条的开列表挑（回执不再弹，交回的那一份去掉回执）；别的带回
    /// `session` 的（`/life new` 开了一条）照它跳过去。
    pub(super) fn follow_ran(&mut self, update: Update) -> Update {
        let Update::CommandRan {
            command,
            said,
            session,
            lives,
        } = update
        else {
            return update;
        };
        if command == "life" && lives.len() > 1 {
            self.pick_life(lives);
            return Update::CommandRan {
                command,
                said: String::new(),
                session,
                lives: Vec::new(),
            };
        }
        if let Some(to) = lives.first().map(|l| l.session.clone()).or(session.clone()) {
            self.switch_session(to);
        }
        Update::CommandRan {
            command,
            said,
            session,
            lives,
        }
    }

    /// 还没开会话时光打 `/life`：不开会话，问 `life.list`，交回 `true`。开着会话的照旧交给核心（`false`）。
    pub(super) fn bare_life(&mut self, line: &str) -> bool {
        if !bare(line) || self.talk.transcript.session.is_some() {
            return false;
        }
        self.link.core.send(Command::Ask {
            tag: LIST_TAG,
            method: "life.list",
            params: json!({}),
        });
        true
    }

    /// 推演人生的几样回应：人生卡上的暂停、继续、结束（`life_card.rs`），还没开会话时的 `life.list`。归这里交回 `true`。
    pub(super) fn life_answer(&mut self, update: &Update) -> bool {
        self.life_card_answer(update) || self.life_list_answer(update)
    }

    /// `life.list` 的回应：同光打 `/life`。一条都没有的说「没有正在过的人生」；拒了的弹原话。回应归这里交回 `true`。
    fn life_list_answer(&mut self, update: &Update) -> bool {
        let Update::Answer { tag, result } = update else {
            return false;
        };
        if *tag != LIST_TAG {
            return false;
        }
        match result {
            Ok(got) => {
                let lives = crate::core::read_lives(&got["lives"]);
                match lives.len() {
                    0 => self.hint(self.config.text.loops.life.none.clone(), false),
                    1 => self.switch_session(lives[0].session.clone()),
                    _ => self.pick_life(lives),
                }
            }
            Err(refused) => self.hint(refused.message.clone(), false),
        }
        true
    }

    /// 开列表：停在正在看的那一条上，没有的停第一条。
    fn pick_life(&mut self, lives: Vec<Life>) {
        let here = self.talk.transcript.session.as_deref();
        let selected = lives
            .iter()
            .position(|l| Some(l.session.as_str()) == here)
            .unwrap_or(0);
        self.boxes.lives = lives;
        self.boxes.panel = Some(Panel::Lives { selected });
    }

    /// 列表开着时的按键：`↑` `↓` 选，`Enter` 跳过去，`Esc` 关。
    pub(super) fn lives_key(&mut self, selected: usize, key: KeyEvent) {
        let last = self.boxes.lives.len().saturating_sub(1);
        let selected = match key.code {
            KeyCode::Up => selected.saturating_sub(1),
            KeyCode::Down => (selected + 1).min(last),
            KeyCode::Enter => return self.choose_life(selected),
            KeyCode::Esc => {
                self.boxes.panel = None;
                return;
            }
            _ => selected,
        };
        self.boxes.panel = Some(Panel::Lives { selected });
    }

    /// 鼠标在列表上：悬停选中，点一下跳过去；交回归不归它。点在框的边上不算点中哪一条。
    pub(super) fn lives_mouse(&mut self, mouse: MouseEvent, at: Position) -> bool {
        let Some(Panel::Lives { .. }) = self.boxes.panel else {
            return false;
        };
        let areas = self.screen.areas;
        if !areas.menu.contains(at) {
            return false;
        }
        let row = at.y.checked_sub(areas.menu_text.y).map(usize::from);
        let index = row.and_then(|row| self.screen.panel_rows.get(row).copied().flatten());
        if let Some(index) = index.filter(|_| areas.menu_text.contains(at)) {
            self.boxes.panel = Some(Panel::Lives { selected: index });
            if matches!(mouse.kind, MouseEventKind::Down(_)) {
                self.choose_life(index);
            }
        }
        true
    }

    /// 跳到第 `index` 条，关列表。
    fn choose_life(&mut self, index: usize) {
        self.boxes.panel = None;
        if let Some(life) = self.boxes.lives.get(index) {
            self.switch_session(life.session.clone());
        }
    }
}
