//! `/sessions`（同终端界面「会话列表」，列表、筛、画法用它的 `session_list`）：选一个当终端会话（`session.set_terminal`），
//! 之后在 shell 里说的话进它。只换指向：置顶、删除这些到终端界面去做。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::text::Line;
use serde_json::json;

use super::{Picked, Picker, result};
use crate::config::Config;
use crate::core::{Command, Core, Update};
use crate::session_list::{SessionList, short};
use crate::shellhook::say::turn::Turn;
use crate::shellhook::words::{Words, fill};
use crate::ui::Chrome;

/// 指过去那一问的编号。
const POINT_TAG: u64 = 91;

/// `/sessions` 的列表。
pub struct SessionsPick {
    list: SessionList,
    words: Words,
}

impl SessionsPick {
    pub fn new(words: &Words) -> Self {
        Self {
            list: SessionList::default(),
            words: words.clone(),
        }
    }
}

impl Picker for SessionsPick {
    fn start(&mut self, core: &Core) {
        core.send(Command::ListSessions);
    }

    fn core(&mut self, update: &Update, _turn: &Turn) {
        if let Update::Sessions(all) = update {
            self.list.replace(all.clone());
        }
    }

    fn ready(&mut self, _turn: &mut Turn, _core: &Core, _config: &Config) -> Picked {
        Picked::Stay
    }

    fn typing(&self) -> bool {
        self.list.typing
    }

    fn key(&mut self, key: KeyEvent, turn: &mut Turn, core: &Core, config: &Config) -> Picked {
        // 搜索那一截照终端界面（按键模式按 `/` 才搜，`list_search`）。
        let query = self.list.query.clone();
        match crate::list_search::key(config.layout.vim, &mut self.list.typing, &query, key) {
            crate::list_search::Step::Query(query) => {
                self.list.typed(query);
                return Picked::Stay;
            }
            crate::list_search::Step::Kept => return Picked::Stay,
            crate::list_search::Step::Pass => {}
        }
        match key.code {
            KeyCode::Esc => return Picked::Done(None),
            KeyCode::Up => self.list.step(false),
            KeyCode::Down => self.list.step(true),
            KeyCode::Enter => {
                let Some(picked) = self.list.picked().cloned() else {
                    return Picked::Stay;
                };
                if turn.transcript.session.as_deref() != Some(picked.session.as_str()) {
                    core.send(Command::Ask {
                        tag: POINT_TAG,
                        method: "session.set_terminal",
                        params: json!({ "session": picked.session }),
                    });
                }
                let title = picked
                    .title
                    .clone()
                    .unwrap_or_else(|| format!("#{}", short(&picked.session)));
                return Picked::Done(Some(result(&fill(
                    &self.words.slash_sessions,
                    &[("title", &title)],
                ))));
            }
            _ => {}
        }
        Picked::Stay
    }

    fn view(
        &self,
        turn: &Turn,
        config: &Config,
        width: u16,
        max: usize,
    ) -> (Chrome, Vec<Line<'static>>) {
        let here = std::env::current_dir()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_default();
        let marks = crate::ui::session_list::Marks {
            current: turn.transcript.session.as_deref(),
            asking: &[],
        };
        let (chrome, lines, _) =
            crate::ui::session_list::lines(&self.list, marks, &here, config, width, max, 0);
        // 提示只写 shell 里有的键：勾选、置顶、删除到终端界面去做（照 `/model` 的几句，按键模式、在搜的各一句）。
        let texts = &config.text.model_panel;
        let hint = crate::list_search::hint(
            config.layout.vim,
            self.list.typing,
            &texts.hint,
            &texts.hint_vim,
            &texts.searching,
        );
        (chrome.hint(hint), lines)
    }
}
