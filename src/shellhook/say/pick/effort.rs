//! `/effort`（同终端界面，排法用它的 `ui::effort_list`）：终端会话用的模型的思考强度，第 0 行是「默认」（照供应商）。
//! 用的是模型池的不让改（同终端界面，2026-10-02 项目主人定）。`/effort 档名` 不弹列表直接改（`default` 是默认）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::text::Line;

use super::{Picked, Picker, result};
use crate::config::Config;
use crate::core::{Command, Core, Efforts, Update};
use crate::shellhook::say::turn::Turn;
use crate::shellhook::words::{Words, fill};
use crate::ui::Chrome;

/// `/effort` 的列表。
pub struct EffortPick {
    /// 那个模型有哪几级；核心还没交回来的是 `None`。
    target: Option<Efforts>,
    /// 选着第几行（第 0 行默认）。
    selected: usize,
    /// 终端会话的最新一页读回来了：用的哪个模型才知道，这时再问。
    paged: bool,
    /// 问过了。
    asked: bool,
    /// 核心说用的是模型池（问不到能改的那个模型）。
    pooled: bool,
    given: Option<String>,
    words: Words,
}

impl EffortPick {
    /// `given` 是 `/effort` 后面的字。
    pub fn new(given: Option<&str>, words: &Words) -> Self {
        Self {
            target: None,
            selected: 0,
            paged: false,
            asked: false,
            pooled: false,
            given: given.map(str::to_string),
            words: words.clone(),
        }
    }

    /// 定下第 `index` 行（第 0 行默认）：和原来的不一样才改。
    fn choose(&self, index: usize, core: &Core, config: &Config) -> Picked {
        let Some(target) = &self.target else {
            return Picked::Stay;
        };
        let level = index
            .checked_sub(1)
            .and_then(|i| target.levels.get(i).cloned());
        if index > 0 && level.is_none() {
            return Picked::Stay;
        }
        if level != target.current {
            core.send(Command::SetEffort {
                key: target.key.clone(),
                level: level.clone(),
            });
        }
        let shown = level.unwrap_or_else(|| config.text.effort.default.clone());
        Picked::Done(Some(result(&fill(
            &self.words.slash_effort,
            &[("level", &shown)],
        ))))
    }
}

impl Picker for EffortPick {
    fn start(&mut self, _core: &Core) {}

    fn core(&mut self, update: &Update, turn: &Turn) {
        match update {
            Update::Paged(_) | Update::Older { .. } => self.paged = true,
            Update::Efforts(list) => {
                let in_use = turn
                    .transcript
                    .model
                    .as_ref()
                    .map(|(m, e)| format!("{e}/{m}"));
                match list.target(in_use.as_deref(), turn.transcript.model_ref()) {
                    Some(target) => {
                        self.selected = crate::ui::effort_list::initial(target);
                        self.target = Some(target.clone());
                    }
                    None => self.pooled = true,
                }
            }
            _ => {}
        }
    }

    fn ready(&mut self, turn: &mut Turn, core: &Core, config: &Config) -> Picked {
        if self.paged && !self.asked {
            self.asked = true;
            let in_use = turn
                .transcript
                .model
                .as_ref()
                .map(|(m, e)| format!("{e}/{m}"));
            let refs = [turn.transcript.model_ref(), in_use.as_deref()];
            core.send(Command::ListEfforts(crate::core::wanted_models(&refs)));
        }
        if self.pooled {
            return Picked::Done(Some(result(&config.text.effort.need_model)));
        }
        let (Some(given), Some(target)) = (&self.given, &self.target) else {
            return Picked::Stay;
        };
        let wanted = given.trim().to_lowercase();
        let index = if wanted == "default" || wanted == config.text.effort.default.to_lowercase() {
            Some(0)
        } else {
            target
                .levels
                .iter()
                .position(|l| l.to_lowercase() == wanted)
                .map(|i| i + 1)
        };
        match index {
            Some(index) => self.choose(index, core, config),
            None => Picked::Done(Some(result(&fill(
                &self.words.slash_no_match,
                &[("query", given)],
            )))),
        }
    }

    fn key(&mut self, key: KeyEvent, _turn: &mut Turn, core: &Core, config: &Config) -> Picked {
        let last = self.target.as_ref().map_or(0, |t| t.levels.len());
        match key.code {
            KeyCode::Esc => return Picked::Done(None),
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => self.selected = (self.selected + 1).min(last),
            KeyCode::Enter => return self.choose(self.selected, core, config),
            _ => {}
        }
        Picked::Stay
    }

    fn view(
        &self,
        _turn: &Turn,
        config: &Config,
        width: u16,
        max: usize,
    ) -> (Chrome, Vec<Line<'static>>) {
        let (chrome, lines, _) =
            crate::ui::effort_list::lines(self.target.as_ref(), self.selected, config, width, max);
        (chrome, lines)
    }
}
