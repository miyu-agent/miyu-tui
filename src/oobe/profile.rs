//! 「AI 怎么认识你」那一步（蓝图 `tui.md`「第一次打开的引导」第 23b 条，2026-10-13 项目主人定，核心 P-9）：三项，
//! 「AI 用什么语言回复」写 `profile.preferred_reply_language`（核心 P-9 补：下拉加「其他」手写，同配置页，
//! `settings/pages/combo.rs`；2026-10-16 项目主人定排第一），「AI 怎么称呼你」写 `profile.name`，「更多描述（可选）」写
//! `profile.about`（长文，在大编辑浮窗里写，`sheet.rs`）。
//! 都写个人设置，都可以不填：一个字没改的不写，清空了原来有的去掉。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::asker::{Asker, Waiting};
use super::field::{Edit, Field};
use super::nav::{self, Nav};
use super::sheet::{Sheet, SheetKey};
use super::{Effect, Move};
use crate::core::Refusal;
use crate::settings::pages::choose::{Choose, Pressed};
use crate::settings::pages::combo;

/// 三项的键，照画的先后（下标是 [`LANGUAGE`]、[`NAME`]、[`ABOUT`]，字表的 `fields`、`hints` 也照这个先后）。
pub const KEYS: [&str; 3] = [
    "profile.preferred_reply_language",
    "profile.name",
    "profile.about",
];
/// 回复语言在第几项。
pub const LANGUAGE: usize = 0;
/// 称呼在第几项。
pub const NAME: usize = 1;
/// 更多描述在第几项。
pub const ABOUT: usize = 2;

/// 光标停的几格。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Slot {
    /// 回复语言。
    #[default]
    Language,
    /// AI 怎么称呼你。
    Name,
    /// 更多描述。
    About,
    /// 「下一步」。
    Next,
}

/// 照画的先后。
pub const SLOTS: [Slot; 4] = [Slot::Language, Slot::Name, Slot::About, Slot::Next];

/// 这一步。
#[derive(Debug)]
pub struct ProfileStep {
    /// AI 怎么称呼你。
    pub name: Field,
    /// 更多描述。
    pub about: Field,
    /// 回复语言：选了建议值的是它，「其他…」手写的在这一格里写。
    pub language: Field,
    /// 回复语言的建议值（`config.schema` 这一项的 `suggestions`）。
    pub suggestions: Vec<String>,
    /// 回复语言的选择窗开着。
    pub picking: Option<Choose>,
    /// 大编辑浮窗开着（写更多描述）。
    pub writing: Option<Sheet>,
    /// 光标在哪一格。
    pub focus: Slot,
    /// 在读、在存。
    pub busy: bool,
    /// 存不成：核心的原话。
    pub error: Option<String>,
    /// 进这一步时读到的三项：存的时候比，没改的不写。
    read: [Option<String>; 3],
    entered: bool,
    in_editor: bool,
}

impl Default for ProfileStep {
    fn default() -> Self {
        Self {
            name: Field::line(),
            about: Field::lines(),
            language: Field::line(),
            suggestions: Vec::new(),
            picking: None,
            writing: None,
            focus: Slot::Language,
            busy: false,
            error: None,
            read: [None, None, None],
            entered: false,
            in_editor: false,
        }
    }
}

impl ProfileStep {
    /// 进这一步：第一次进的先读现在写的（重走引导时照它填好）。
    pub fn enter(&mut self, asker: &mut Asker) {
        if self.entered {
            return;
        }
        self.entered = true;
        self.busy = true;
        asker.ask("config.get", json!({"keys": KEYS}), Waiting::ProfileRead);
        // 回复语言的建议值照配置清单（字表照连接的语言给）。
        asker.ask("config.schema", json!({}), Waiting::ProfileSchema);
    }

    /// 按了一个键；`words` 是回复语言选择窗的「不设」「其他…」。
    pub fn key(
        &mut self,
        key: KeyEvent,
        asker: &mut Asker,
        effects: &mut Vec<Effect>,
        words: [&str; 2],
    ) -> Move {
        if let Some(mut choose) = self.picking.take() {
            match choose.press(key) {
                Pressed::Picked(value) => self.picked(&value),
                Pressed::Cancelled => {}
                Pressed::Moved | Pressed::Ignored => self.picking = Some(choose),
            }
            return Move::Stay;
        }
        if let Some(mut sheet) = self.writing.take() {
            match sheet.key(key) {
                SheetKey::Close => {
                    self.about = Field::lines().with(sheet.text());
                    return Move::Stay;
                }
                SheetKey::External => {
                    self.in_editor = true;
                    effects.push(Effect::Editor(sheet.text().to_string()));
                }
                SheetKey::Stay => {}
            }
            self.writing = Some(sheet);
            return Move::Stay;
        }
        if self.busy {
            return if key.code == KeyCode::Esc {
                Move::Back
            } else {
                Move::Stay
            };
        }
        // 称呼那一格 `Enter` 才开始编辑，编辑时 `Enter` 写好、`Esc` 不改（第 7 条）。
        if self.name.editing() {
            if self.name.edit(key) == Edit::Done {
                self.error = None;
            }
            return Move::Stay;
        }
        // 「其他…」手写回复语言：同称呼那一格。
        if self.language.editing() {
            self.language.edit(key);
            return Move::Stay;
        }
        let at = SLOTS.iter().position(|s| *s == self.focus).unwrap_or(0);
        let moved = nav::plain(key);
        match key.code {
            KeyCode::Esc => return Move::Back,
            _ if moved == Some(Nav::Up) => self.focus = SLOTS[at.saturating_sub(1)],
            _ if moved == Some(Nav::Down) => self.focus = SLOTS[(at + 1).min(SLOTS.len() - 1)],
            KeyCode::Enter => match self.focus {
                Slot::Name => self.name.begin(),
                Slot::About => self.writing = Some(Sheet::open(self.about.text())),
                Slot::Language => {
                    let now = self.language_value();
                    let rows = combo::rows(&self.suggestions, words);
                    let at = combo::landing(&self.suggestions, &now);
                    self.picking = Some(Choose::open(KEYS[LANGUAGE], "", rows, &at));
                }
                Slot::Next => return self.save(asker),
            },
            _ => {}
        }
        Move::Stay
    }

    /// 回复语言现在写的；空的是没设。
    fn language_value(&self) -> Value {
        let text = self.language.trimmed();
        if text.is_empty() {
            Value::Null
        } else {
            Value::String(text)
        }
    }

    /// 选择窗选好了：建议值、「不设」当场换上；「其他…」在这一格里手写（原来写的不在建议里的留着）。
    fn picked(&mut self, value: &Value) {
        if *value == combo::other() {
            if self.suggestions.contains(&self.language.trimmed()) {
                self.language = Field::line();
            }
            self.language.begin();
            return;
        }
        self.language = Field::line().with(value.as_str().unwrap_or_default());
    }

    /// 三项现在写的，照 [`KEYS`] 的先后。
    fn now(&self) -> [String; 3] {
        let mut now = [const { String::new() }; 3];
        now[LANGUAGE] = self.language.trimmed();
        now[NAME] = self.name.trimmed();
        now[ABOUT] = self.about.text().trim().to_string();
        now
    }

    /// 三项都空着：「下一步」写成「跳过」，点了不存。
    pub fn blank(&self) -> bool {
        self.now().iter().all(String::is_empty)
    }

    /// 「下一步」：改了的写进个人设置（清空了原来有的去掉），一个字没改的直接下一步。
    fn save(&mut self, asker: &mut Asker) -> Move {
        // 三项都空着的是「跳过」：什么都不写（原来有的也不去掉，2026-10-19 项目主人）。
        if self.blank() {
            return Move::Next;
        }
        let fields = self.now();
        let changes: Vec<Value> = KEYS
            .iter()
            .zip(fields)
            .zip(&self.read)
            .filter_map(|((key, now), was)| match (now.is_empty(), was) {
                (true, Some(_)) => Some(json!({"key": key, "unset": true})),
                (true, None) => None,
                (false, Some(was)) if *was == now => None,
                (false, _) => Some(json!({"key": key, "value": now})),
            })
            .collect();
        if changes.is_empty() {
            return Move::Next;
        }
        self.error = None;
        self.busy = true;
        asker.ask(
            "config.set",
            json!({"layer": "personal", "changes": changes}),
            Waiting::ProfileSave,
        );
        Move::Stay
    }

    /// 回应：读到的填进两格；存成了下一步，存不成的写原话、留在这一步。
    pub fn answer(&mut self, waiting: &Waiting, result: Result<Value, Refusal>) -> Move {
        self.busy = false;
        match waiting {
            Waiting::ProfileRead => {
                let got = result.ok().unwrap_or_default();
                for (i, key) in KEYS.iter().enumerate() {
                    let value = got["items"][*key]["value"]
                        .as_str()
                        .filter(|v| !v.is_empty())
                        .map(str::to_string);
                    if let Some(text) = &value {
                        match i {
                            LANGUAGE => self.language = Field::line().with(text),
                            NAME => self.name = Field::line().with(text),
                            _ => self.about = Field::lines().with(text),
                        }
                    }
                    self.read[i] = value;
                }
                Move::Stay
            }
            Waiting::ProfileSave => match result {
                Ok(_) => {
                    self.read = self.now().map(|t| Some(t).filter(|t| !t.is_empty()));
                    Move::Next
                }
                Err(refusal) => {
                    self.error = Some(crate::settings::pages::target::said(&refusal));
                    Move::Stay
                }
            },
            Waiting::ProfileSchema => {
                let got = result.ok().unwrap_or_default();
                self.suggestions = got["items"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|i| i["key"] == KEYS[LANGUAGE])
                    .and_then(|i| i["suggestions"].as_array())
                    .into_iter()
                    .flatten()
                    .filter_map(|s| s.as_str().map(str::to_string))
                    .collect();
                Move::Stay
            }
            _ => Move::Stay,
        }
    }

    /// 粘贴：大编辑浮窗开着的进浮窗，光标在称呼那一格的进那一格。
    pub fn paste(&mut self, text: &str) {
        if let Some(sheet) = self.writing.as_mut() {
            sheet.paste(text);
        } else if self.language.editing() {
            self.language.paste(text);
        } else if self.focus == Slot::Name {
            if !self.name.editing() {
                self.name.begin();
            }
            self.name.paste(text);
        }
    }

    /// 外部编辑器退出了：写回大编辑浮窗（没改、读不回来的不动）。
    pub fn edited(&mut self, text: Option<String>) {
        if std::mem::take(&mut self.in_editor)
            && let (Some(text), Some(sheet)) = (text, self.writing.as_mut())
        {
            sheet.replace(text.trim_end());
        }
    }

    /// 断开了，读的没回来：不再忙，下次进这一步再读。
    pub fn lost(&mut self) {
        if self.busy {
            self.busy = false;
            self.entered = false;
        }
    }
}
