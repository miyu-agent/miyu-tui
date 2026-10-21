//! `/persona`、`/preset`（设计 22 第七节「终端会话」：终端会话用哪位人格、哪个预设是 `terminal.persona`、`terminal.preset`）：
//! 列表、按键、画法用终端界面的选择框（`pick_box`、`ui::persona_list`，2026-10-20 和终端界面会话对的：选了写到哪由用它的
//! 一方定）。选了写进个人设置，之后新开的终端会话照它；正在说的这一个不换。`/persona 名字` 不弹列表直接写。
//! 按 `e` 编辑那一项要整个配置页，shell 里先不做。

use ratatui::crossterm::event::KeyEvent;
use ratatui::text::Line;
use serde_json::json;

use super::{Picked, Picker, result};
use crate::app::new_session::Kind;
use crate::config::Config;
use crate::core::{Command, Core, Persona, Update};
use crate::pick_box::{Pressed, press};
use crate::shellhook::say::turn::Turn;
use crate::shellhook::words::{Words, fill};
use crate::ui::Chrome;

/// 读 `terminal.*` 那一问的编号。
const READ_TAG: u64 = 93;
/// 写那一问的编号。
const WRITE_TAG: u64 = 94;

/// `/persona`、`/preset` 的列表。
pub struct PersonaPick {
    kind: Kind,
    /// 一行行（人格的第一行是「无人格」）；核心还没交回来的是 `None`。
    list: Option<Vec<Persona>>,
    /// 默认的那一个（`HeadConfig`）。
    default: Option<String>,
    /// 现在的 `terminal.persona` / `terminal.preset`；没设的（跟随默认）是 `None`。
    current: Option<String>,
    /// 读回来了。
    read: bool,
    selected: usize,
    given: Option<String>,
    words: Words,
    /// 「无人格」那一行（同终端界面 `new_session.rs` 的 `listed`）。
    none: Persona,
}

impl PersonaPick {
    pub fn new(kind: Kind, given: Option<&str>, words: &Words, config: &Config) -> Self {
        let texts = &config.text.persona_box;
        Self {
            none: Persona {
                id: String::new(),
                name: Some(texts.none.clone()),
                summary: Some(texts.none_note.clone()),
                problem: None,
                avatar: None,
                seed: None,
                seeds: None,
            },
            kind,
            list: None,
            default: None,
            current: None,
            read: false,
            selected: 0,
            given: given.map(str::to_string),
            words: words.clone(),
        }
    }

    /// 写到哪一项。
    fn key_name(&self) -> &'static str {
        match self.kind {
            Kind::Persona => "terminal.persona",
            Kind::Preset => "terminal.preset",
        }
    }

    /// 列表和设置都读回来了：停到现在用的（没设的照默认）上。
    fn place(&mut self) {
        let Some(list) = &self.list else { return };
        let wanted = self.current.as_deref().or(self.default.as_deref());
        self.selected = list
            .iter()
            .position(|p| Some(p.id.as_str()) == wanted)
            .or_else(|| crate::pick_box::usable(list).next())
            .unwrap_or(0);
    }

    /// 选定第 `index` 个：写进个人设置，留一行。
    fn choose(&self, index: usize, core: &Core) -> Picked {
        let Some(row) = self.list.as_ref().and_then(|l| l.get(index)) else {
            return Picked::Stay;
        };
        if self.current.as_deref() != Some(row.id.as_str()) {
            core.send(Command::Ask {
                tag: WRITE_TAG,
                method: "config.set",
                params: json!({
                    "layer": "personal",
                    "changes": [{ "key": self.key_name(), "value": row.id }],
                }),
            });
        }
        let template = match self.kind {
            Kind::Persona => &self.words.slash_persona,
            Kind::Preset => &self.words.slash_preset,
        };
        Picked::Done(Some(result(&fill(template, &[("name", row.label())]))))
    }
}

impl Picker for PersonaPick {
    fn start(&mut self, core: &Core) {
        core.send(match self.kind {
            Kind::Persona => Command::ListPersonas,
            Kind::Preset => Command::ListPresets,
        });
        core.send(Command::Ask {
            tag: READ_TAG,
            method: "config.get",
            params: json!({ "keys": [self.key_name()] }),
        });
    }

    fn core(&mut self, update: &Update, _turn: &Turn) {
        match (update, self.kind) {
            (Update::Personas(list), Kind::Persona) => {
                let mut list = list.clone();
                // 「无人格」一行（同终端界面的人格框），编号是空的。
                list.insert(0, self.none.clone());
                self.list = Some(list);
                self.place();
            }
            (Update::Presets(list), Kind::Preset) => {
                self.list = Some(list.clone());
                self.place();
            }
            (Update::HeadConfig(head), _) => {
                self.default = match self.kind {
                    Kind::Persona => head.persona.clone(),
                    Kind::Preset => head.preset.clone(),
                };
                self.place();
            }
            (Update::Answer { tag, result }, _) if *tag == READ_TAG => {
                self.read = true;
                self.current = result.as_ref().ok().and_then(|got| {
                    got["items"][self.key_name()]["value"]
                        .as_str()
                        .map(str::to_string)
                });
                self.place();
            }
            _ => {}
        }
    }

    fn ready(&mut self, _turn: &mut Turn, core: &Core, _config: &Config) -> Picked {
        let (Some(given), Some(list)) = (&self.given, &self.list) else {
            return Picked::Stay;
        };
        if !self.read {
            return Picked::Stay;
        }
        let wanted = given.trim().to_lowercase();
        let found = list.iter().position(|p| {
            p.problem.is_none()
                && (p.id.to_lowercase() == wanted || p.label().to_lowercase() == wanted)
        });
        match found {
            Some(index) => self.choose(index, core),
            None => Picked::Done(Some(result(&fill(
                &self.words.slash_no_match,
                &[("query", given)],
            )))),
        }
    }

    fn key(&mut self, key: KeyEvent, _turn: &mut Turn, core: &Core, _config: &Config) -> Picked {
        let Some(list) = &self.list else {
            return match key.code {
                ratatui::crossterm::event::KeyCode::Esc => Picked::Done(None),
                _ => Picked::Stay,
            };
        };
        match press(list, self.selected, key) {
            Pressed::Moved(at) => self.selected = at,
            Pressed::Chose(at) => return self.choose(at, core),
            Pressed::Escape => return Picked::Done(None),
            Pressed::Edit(_) | Pressed::CantEdit | Pressed::Ignored => {}
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
        let list = self.list.as_deref().unwrap_or_default();
        let current = self.current.as_deref().or(self.default.as_deref());
        let (chrome, lines, _) = crate::ui::persona_list::lines(
            list,
            self.default.as_deref(),
            (current, false),
            &config.layout.current_mark,
            self.kind,
            self.selected,
            &config.text.persona_box,
            width,
            max,
        );
        // 提示只写 shell 里有的键：`e` 编辑没做（同 `/theme` 那一句）。
        (chrome.hint(&config.text.themes.hint), lines)
    }
}
