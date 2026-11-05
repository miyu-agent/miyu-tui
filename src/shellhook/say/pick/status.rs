//! `/status`（同终端界面「会话状态 `/status`」，排法用它的 `ui::status_box`）：终端会话的状态印成一个框留在滚动区，不交互。
//! 数照终端会话的最新一页填；终端界面才有的（连同子代理的用量、花费）不写，那一行就不印。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::text::{Line, Span};

use super::{Picked, Picker};
use crate::config::Config;
use crate::core::{Core, Update};
use crate::shellhook::say::turn::Turn;
use crate::ui::Chrome;
use crate::ui::status_box::Facts;

/// 最新一页读回来以后，订阅的回应（预设、窗口、压缩线）最多再等多久。
const SUBSCRIBED_WAIT: std::time::Duration = std::time::Duration::from_millis(500);

/// `/status`：等终端会话读回来、订阅上，印一个框。
#[derive(Default)]
pub struct StatusShow {
    /// 终端会话的最新一页什么时候读回来的。
    paged: Option<std::time::Instant>,
    /// 订阅的回应到了：预设、窗口、压缩线跟着它来，紧跟在最新一页后面（实测：一看到最新一页就印，这几行没有）。
    subscribed: bool,
    /// 预设一行行：预设照名字写。
    presets: Vec<crate::core::Persona>,
}

impl StatusShow {
    pub fn new() -> Self {
        Self::default()
    }
}

/// 照终端会话收齐要写的（同 `status_box::facts`，只是数从这里来）。
fn facts(turn: &Turn, presets: &[crate::core::Persona], config: &Config) -> Facts {
    let text = &config.text;
    let t = &turn.transcript;
    let persona = t.persona.as_deref().map(|id| {
        if id.is_empty() {
            text.persona_box.none.clone()
        } else {
            turn.look.persona_name(id).unwrap_or_else(|| id.to_string())
        }
    });
    let cwd = std::env::current_dir()
        .map(|d| d.to_string_lossy().into_owned())
        .unwrap_or_default();
    Facts {
        title: t.title.clone().unwrap_or_else(|| text.untitled.clone()),
        session: t.session.clone(),
        persona,
        preset: t.preset.as_deref().map(|id| {
            presets
                .iter()
                .find(|p| p.id == id)
                .map_or_else(|| id.to_string(), |p| p.label().to_string())
        }),
        level: text.levels.get(&t.level).cloned(),
        cwd: crate::local::home_short(&cwd),
        model: t.shown_model(),
        effort: t.effort().map(str::to_string),
        window: t.limits.window,
        speed: t.speed.average(),
        context: t.context,
        line: t.limits.compaction_line,
        compactions: t.cache.compactions,
        breaks: t.cache.breaks,
        usage: t.total,
        spent: None,
        models: Vec::new(),
        memory: crate::ui::status_box::memory_calls(t),
    }
}

impl Picker for StatusShow {
    fn start(&mut self, core: &Core) {
        core.send(crate::core::Command::ListPresets);
    }

    fn core(&mut self, update: &Update, _turn: &Turn) {
        match update {
            Update::Paged(_) | Update::Older { .. } => {
                self.paged.get_or_insert_with(std::time::Instant::now);
            }
            Update::Snapshot(_) | Update::Limits(_) => self.subscribed = true,
            Update::Presets(list) => self.presets = list.clone(),
            _ => {}
        }
    }

    fn ready(&mut self, turn: &mut Turn, _core: &Core, config: &Config) -> Picked {
        // 配色、人格的名字也等读回来（印出去就改不了了）。
        let Some(paged) = self.paged else {
            return Picked::Stay;
        };
        if !turn.look.ready() || (!self.subscribed && paged.elapsed() < SUBSCRIBED_WAIT) {
            return Picked::Stay;
        }
        let f = facts(turn, &self.presets, config);
        let width = super::super::session::size().0.saturating_sub(4);
        let chrome = Chrome {
            title: vec![Span::raw(f.title.clone())],
            hint: None,
        };
        Picked::Show(chrome, crate::ui::status_box::rows(&f, config, width))
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
