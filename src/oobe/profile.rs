//! 「AI 怎么认识你」那一步（蓝图 `tui.md`「第一次打开的引导」第 23b 条，2026-10-13 项目主人定，核心 P-9）：两项，
//! 「AI 怎么称呼你」写 `profile.name`，「更多描述（可选）」写 `profile.about`（长文，在大编辑浮窗里写，`sheet.rs`）。都写
//! 个人设置、新会话生效，都可以不填：一个字没改的不写，清空了原来有的去掉。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::asker::{Asker, Waiting};
use super::field::{Edit, Field};
use super::nav::{self, Nav};
use super::sheet::{Sheet, SheetKey};
use super::{Effect, Move};
use crate::core::Refusal;

/// 两项的键，照画的先后。
pub const KEYS: [&str; 2] = ["profile.name", "profile.about"];

/// 光标停的几格。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Slot {
    /// AI 怎么称呼你。
    #[default]
    Name,
    /// 更多描述。
    About,
    /// 「下一步」。
    Next,
}

/// 照画的先后。
pub const SLOTS: [Slot; 3] = [Slot::Name, Slot::About, Slot::Next];

/// 这一步。
#[derive(Debug)]
pub struct ProfileStep {
    /// AI 怎么称呼你。
    pub name: Field,
    /// 更多描述。
    pub about: Field,
    /// 大编辑浮窗开着（写更多描述）。
    pub writing: Option<Sheet>,
    /// 光标在哪一格。
    pub focus: Slot,
    /// 在读、在存。
    pub busy: bool,
    /// 存不成：核心的原话。
    pub error: Option<String>,
    /// 进这一步时读到的两项：存的时候比，没改的不写。
    read: [Option<String>; 2],
    entered: bool,
    in_editor: bool,
}

impl Default for ProfileStep {
    fn default() -> Self {
        Self {
            name: Field::line(),
            about: Field::lines(),
            writing: None,
            focus: Slot::Name,
            busy: false,
            error: None,
            read: [None, None],
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
    }

    /// 按了一个键。
    pub fn key(&mut self, key: KeyEvent, asker: &mut Asker, effects: &mut Vec<Effect>) -> Move {
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
        let at = SLOTS.iter().position(|s| *s == self.focus).unwrap_or(0);
        let moved = nav::plain(key);
        match key.code {
            KeyCode::Esc => return Move::Back,
            _ if moved == Some(Nav::Up) => self.focus = SLOTS[at.saturating_sub(1)],
            _ if moved == Some(Nav::Down) => self.focus = SLOTS[(at + 1).min(SLOTS.len() - 1)],
            KeyCode::Enter => match self.focus {
                Slot::Name => self.name.begin(),
                Slot::About => self.writing = Some(Sheet::open(self.about.text())),
                Slot::Next => return self.save(asker),
            },
            _ => {}
        }
        Move::Stay
    }

    /// 「下一步」：改了的写进个人设置（清空了原来有的去掉），一个字没改的直接下一步。
    fn save(&mut self, asker: &mut Asker) -> Move {
        let fields = [self.name.trimmed(), self.about.text().trim().to_string()];
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
                            0 => self.name = Field::line().with(text),
                            _ => self.about = Field::lines().with(text),
                        }
                    }
                    self.read[i] = value;
                }
                Move::Stay
            }
            Waiting::ProfileSave => match result {
                Ok(_) => {
                    self.read = [
                        Some(self.name.trimmed()).filter(|t| !t.is_empty()),
                        Some(self.about.text().trim().to_string()).filter(|t| !t.is_empty()),
                    ];
                    Move::Next
                }
                Err(refusal) => {
                    self.error = Some(crate::settings::pages::target::said(&refusal));
                    Move::Stay
                }
            },
            _ => Move::Stay,
        }
    }

    /// 粘贴：大编辑浮窗开着的进浮窗，光标在称呼那一格的进那一格。
    pub fn paste(&mut self, text: &str) {
        if let Some(sheet) = self.writing.as_mut() {
            sheet.paste(text);
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
