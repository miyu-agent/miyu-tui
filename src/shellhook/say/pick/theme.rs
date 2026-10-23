//! `/theme`（同终端界面「主题」第 6 条，排法用它的 `ui::themes`）：挪到哪种当场换成哪种的样子，`Enter` 写进个人设置的
//! `tui.theme`（和终端界面共一项），`Esc` 换回原来的。`/theme 名字` 不弹列表：值（`terminal`）、照界面语言的名字（「终端
//! 配色」）都认。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::text::Line;

use super::{Picked, Picker, result};
use crate::config::Config;
use crate::core::{Command, Core, Update};
use crate::shellhook::say::turn::Turn;
use crate::shellhook::words::{Words, fill};
use crate::theme::choice::COLORS;
use crate::ui::Chrome;

/// `/theme` 的列表。
pub struct ThemePick {
    /// 选着第几种。
    selected: usize,
    /// 打开时用的那种：`Esc` 换回它；配置还没读回来的是 `None`。
    before: Option<String>,
    /// 带了参数的：照它找，配置读回来就直接定。
    given: Option<String>,
    words: Words,
}

impl ThemePick {
    /// `given` 是 `/theme` 后面的字。
    pub fn new(given: Option<&str>, words: &Words) -> Self {
        Self {
            selected: 0,
            before: None,
            given: given.map(str::to_string),
            words: words.clone(),
        }
    }

    /// 定下 `value`：和原来的不一样才写设置；留一行写成照界面语言的名字。
    fn choose(&self, value: &str, core: &Core, config: &Config) -> Picked {
        if self.before.as_deref() != Some(value) {
            core.send(Command::SetTheme(value.to_string()));
        }
        Picked::Done(Some(result(&fill(
            &self.words.slash_theme,
            &[("name", &name(value, config))],
        ))))
    }
}

/// 配色照界面语言的名字（配置页同一份，`settings.more.choice_names`）。
fn name(value: &str, config: &Config) -> String {
    config
        .text
        .settings
        .more
        .choice_names
        .get("tui.theme")
        .and_then(|n| n.get(value))
        .cloned()
        .unwrap_or_else(|| value.to_string())
}

impl Picker for ThemePick {
    fn needs_session(&self) -> bool {
        false
    }

    fn start(&mut self, _core: &Core) {}

    fn core(&mut self, _update: &Update, _turn: &Turn) {}

    fn ready(&mut self, turn: &mut Turn, core: &Core, config: &Config) -> Picked {
        if !turn.look.ready() {
            return Picked::Stay;
        }
        if self.before.is_none() {
            let before = turn.look.colors().to_string();
            self.selected = COLORS.iter().position(|c| *c == before).unwrap_or(0);
            self.before = Some(before);
        }
        let Some(given) = &self.given else {
            return Picked::Stay;
        };
        let wanted = given.trim().to_lowercase();
        let found = COLORS
            .iter()
            .find(|c| **c == wanted || name(c, config).to_lowercase() == wanted);
        match found {
            Some(value) => self.choose(value, core, config),
            None => Picked::Done(Some(result(&fill(
                &self.words.slash_no_match,
                &[("query", given)],
            )))),
        }
    }

    fn key(&mut self, key: KeyEvent, turn: &mut Turn, core: &Core, config: &Config) -> Picked {
        let Some(before) = self.before.clone() else {
            return match key.code {
                KeyCode::Esc => Picked::Done(None),
                _ => Picked::Stay,
            };
        };
        let last = COLORS.len().saturating_sub(1);
        match key.code {
            KeyCode::Esc => {
                turn.look.preview(&before);
                return Picked::Done(None);
            }
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => self.selected = (self.selected + 1).min(last),
            KeyCode::Enter => {
                if let Some(value) = COLORS.get(self.selected) {
                    return self.choose(value, core, config);
                }
            }
            _ => return Picked::Stay,
        }
        // 挪到哪种当场换成它的样子（同终端界面）。
        if let Some(value) = COLORS.get(self.selected) {
            turn.look.preview(value);
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
        // 带了参数的不弹框，等回来直接定（2026-10-21 项目主人：原来先画一下框又收掉，看着像开了一下终端界面）。
        if self.given.is_some() {
            return (Chrome::new("", Vec::new()), Vec::new());
        }
        let before = self.before.as_deref().unwrap_or_default();
        let (chrome, lines, _) =
            crate::ui::themes::lines(config, (&COLORS[..], before), self.selected, width, max);
        (chrome, lines)
    }
}
