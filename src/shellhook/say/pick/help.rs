//! `/help`（同终端界面，排法用它的 `ui::help`）：帮助框印在滚动区，不交互（2026-10-21 项目主人：「可以和 tui 里面一样
//! 用 tui 展示帮助信息的吧？」）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::text::Line;

use super::{Picked, Picker};
use crate::config::Config;
use crate::core::{Core, Update};
use crate::shellhook::say::turn::Turn;
use crate::ui::Chrome;

/// `/help`：配色、给人看的字读回来就印。
#[derive(Default)]
pub struct HelpShow;

impl Picker for HelpShow {
    fn needs_session(&self) -> bool {
        false
    }

    fn start(&mut self, _core: &Core) {}

    fn core(&mut self, _update: &Update, _turn: &Turn) {}

    fn ready(&mut self, turn: &mut Turn, _core: &Core, config: &Config) -> Picked {
        if !turn.look.ready() {
            return Picked::Stay;
        }
        let width = super::super::session::size().0.saturating_sub(4);
        let (chrome, lines) = crate::ui::help::lines(config, width, 0, usize::MAX);
        Picked::Show(
            Chrome {
                hint: None,
                ..chrome
            },
            lines,
        )
    }

    fn key(&mut self, key: KeyEvent, _turn: &mut Turn, _core: &Core, _config: &Config) -> Picked {
        match key.code {
            KeyCode::Esc => Picked::Done(None),
            _ => Picked::Stay,
        }
    }

    fn view(
        &self,
        _turn: &Turn,
        _config: &Config,
        _width: u16,
        _max: usize,
    ) -> (Chrome, Vec<Line<'static>>) {
        (Chrome::new("", Vec::new()), Vec::new())
    }
}
