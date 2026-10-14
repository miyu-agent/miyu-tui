//! 主题那一步（蓝图 `tui.md`「第一次打开的引导」第 23a 条，2026-10-13 项目主人定，版面 2026-10-14 定）：明暗、配色两行，
//! 在一行上按 `Enter` 开选择窗（和配置页选值的同一个、同一套键，2026-10-14 项目主人：以后主题多了、有别人做的，左右切不
//! 好用），上下挪的时候整个界面当场换成挪到的样子（App 每一帧照 [`ThemeStep::preview`] 挑，`app/theme.rs`），`Enter` 选好，
//! `Esc` 换回开窗以前的。「下一步 →」写个人设置的 `tui.theme_mode`、`tui.theme`，`Esc` 回上一步、换回原来的样子。配色没
//! 写过、人格那一步有头像（或写了主题色）的，进来停在「跟随头像」。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::Move;
use super::asker::{Asker, Waiting};
use super::nav::{self, Nav};
use super::texts::Theme as Words;
use crate::core::Refusal;
use crate::settings::pages::choose::{Choice, Choose, Pressed};
use crate::theme::choice::{COLORS, Mode};

/// 明暗三档，选择窗照这个先后列；配置里的写法同下标。
pub const MODES: [Mode; 3] = [Mode::Auto, Mode::Dark, Mode::Light];
const MODE_NAMES: [&str; 3] = ["auto", "dark", "light"];

/// 光标停在哪一行。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Slot {
    /// 明暗。
    #[default]
    Mode,
    /// 配色。
    Colors,
    /// 「下一步 →」。
    Next,
}

/// 这一步。
#[derive(Debug, Default)]
pub struct ThemeStep {
    /// 光标停在哪一行。
    pub slot: Slot,
    /// 明暗选着第几档（[`MODES`]）。
    pub mode: usize,
    /// 配色选着第几种（`choice::COLORS`）。
    pub colors: usize,
    /// 在读、在写：按键先不认（`Esc` 照常回上一步）。
    pub busy: bool,
    /// 写不成：核心的原话。
    pub error: Option<String>,
    /// 写成了：引导关上以前一直照选的（配置变了的推送回来以前不闪回原来的样子）。
    pub saved: bool,
    /// 读到了现在的配置：从这时起照选的盖（读到以前照原来的样子）。
    read: bool,
    /// 配色哪一层都没写（`config.get` 的来源是默认值）。
    unset: bool,
    /// 这一回选过配色：不再替人停到「跟随头像」。
    turned: bool,
    entered: bool,
    /// 开着的选择窗（明暗、配色那一行按 `Enter` 开的）。
    pub picking: Option<Choose>,
    /// 开窗以前选着第几个：`Esc` 换回它。
    before: usize,
    /// 找到了能取色的桌面壁纸：配色列表里列「跟随壁纸」（App 每一拍照 `theme/wallpaper.rs` 设）。
    pub wallpaper: bool,
}

impl ThemeStep {
    /// 进这一步：第一次进的先读现在写的（重走引导时照它停）。`follow` 是人格那一步有没有能取色的（头像、主题色）：配色没
    /// 写过、没换过的停在「跟随头像」。
    pub fn enter(&mut self, asker: &mut Asker, follow: bool) {
        if !self.entered {
            self.entered = true;
            self.busy = true;
            let keys = json!({"keys": ["tui.theme", "tui.theme_mode"]});
            asker.ask("config.get", keys, Waiting::ThemeRead);
            return;
        }
        self.settle(follow);
    }

    /// 配色没写过、没换过、有能取色的：停在「跟随头像」。
    fn settle(&mut self, follow: bool) {
        if follow && self.unset && !self.turned {
            self.colors = 0;
        }
    }

    /// 按了一个键：开着选择窗的归它（挪了当场换样子，`Esc` 换回开窗以前的）；不然上下换行，明暗、配色那一行 `Enter` 开
    /// 选择窗，「下一步」那一行 `Enter` 写，`Esc` 回上一步。
    pub fn key(&mut self, key: KeyEvent, asker: &mut Asker, words: &Words) -> Move {
        if let Some(choose) = &mut self.picking {
            match choose.press(key) {
                Pressed::Moved => {
                    let value = choose.chosen().cloned();
                    self.set_value(value.as_ref());
                }
                Pressed::Picked(value) => {
                    self.set_value(Some(&value));
                    self.picking = None;
                }
                Pressed::Cancelled => {
                    let before = self.before;
                    self.set(before);
                    self.picking = None;
                }
                Pressed::Ignored => {}
            }
            return Move::Stay;
        }
        if key.code == KeyCode::Esc {
            return Move::Back;
        }
        if self.busy {
            return Move::Stay;
        }
        match nav::plain(key) {
            Some(Nav::Up) => {
                self.slot = match self.slot {
                    Slot::Next => Slot::Colors,
                    _ => Slot::Mode,
                }
            }
            Some(Nav::Down) => {
                self.slot = match self.slot {
                    Slot::Mode => Slot::Colors,
                    _ => Slot::Next,
                }
            }
            None if key.code == KeyCode::Enter => match self.slot {
                Slot::Next => self.save(asker),
                _ => self.open(words),
            },
            _ => {}
        }
        Move::Stay
    }

    /// 光标那一行开选择窗：一个选项一行，停在选着的那个上。
    fn open(&mut self, words: &Words) {
        let (key, title, names, values, at): (_, _, &[String], &[&str], _) = match self.slot {
            Slot::Mode => (
                "tui.theme_mode",
                &words.rows[0],
                &words.modes,
                &MODE_NAMES,
                self.mode,
            ),
            Slot::Colors => (
                "tui.theme",
                &words.rows[1],
                &words.colors,
                &COLORS,
                self.colors,
            ),
            Slot::Next => return,
        };
        // 找不到壁纸的不列「跟随壁纸」（「主题」的「跟随壁纸」）。
        let wallpaper = self.wallpaper;
        let choices = names
            .iter()
            .zip(values)
            .filter(|(_, value)| wallpaper || **value != "wallpaper")
            .map(|(name, value)| Choice {
                value: json!(value),
                label: name.clone(),
                note: None,
                off: None,
            })
            .collect();
        self.before = at;
        self.picking = Some(Choose::open(key, title, choices, &json!(values[at])));
    }

    /// 光标那一行选成值是 `value` 的那一个（列表里少了跟随壁纸的，不照第几行算）。
    fn set_value(&mut self, value: Option<&Value>) {
        let Some(value) = value.and_then(Value::as_str) else {
            return;
        };
        let at = match self.slot {
            Slot::Mode => MODE_NAMES.iter().position(|v| *v == value),
            Slot::Colors => COLORS.iter().position(|v| *v == value),
            Slot::Next => None,
        };
        if let Some(at) = at {
            self.set(at);
        }
    }

    /// 光标那一行选成第 `at` 个：当场换样子。
    fn set(&mut self, at: usize) {
        match self.slot {
            Slot::Mode => self.mode = at.min(MODES.len() - 1),
            Slot::Colors => {
                self.colors = at.min(COLORS.len() - 1);
                self.turned = true;
            }
            Slot::Next => return,
        }
        self.saved = false;
    }

    fn save(&mut self, asker: &mut Asker) {
        let params = json!({"layer": "personal", "changes": [
            {"key": "tui.theme_mode", "value": MODE_NAMES[self.mode]},
            {"key": "tui.theme", "value": COLORS[self.colors]},
        ]});
        self.error = None;
        self.busy = true;
        asker.ask("config.set", params, Waiting::ThemeSave);
    }

    /// 回应：读到的照它停；写成了下一步，写不成的写原话、留在这一步。
    pub fn answer(
        &mut self,
        waiting: &Waiting,
        result: Result<Value, Refusal>,
        follow: bool,
    ) -> Move {
        self.busy = false;
        match waiting {
            Waiting::ThemeRead => {
                let got = result.unwrap_or_default();
                let items = &got["items"];
                let colors = items["tui.theme"]["value"].as_str();
                if let Some(at) = colors.and_then(|c| COLORS.iter().position(|v| *v == c)) {
                    self.colors = at;
                }
                let mode = items["tui.theme_mode"]["value"]
                    .as_str()
                    .and_then(Mode::read);
                if let Some(at) = mode.and_then(|m| MODES.iter().position(|v| *v == m)) {
                    self.mode = at;
                }
                let layer = items["tui.theme"]["origin"]["layer"].as_str();
                self.unset = layer.is_none_or(|l| l == "default");
                self.read = true;
                self.settle(follow);
                Move::Stay
            }
            Waiting::ThemeSave => match result {
                Ok(_) => {
                    self.saved = true;
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

    /// 选着的样子：配色、明暗；还没读到现在的配置的是 `None`（照原来的样子）。
    pub fn preview(&self) -> Option<(String, Mode)> {
        self.read
            .then(|| (COLORS[self.colors].to_string(), MODES[self.mode]))
    }

    /// 在写（读到配置以后才会忙的只有写）。
    pub fn saving(&self) -> bool {
        self.busy && self.read
    }

    /// 断开了，读的没回来：不再忙，下次进这一步再读。
    pub fn lost(&mut self) {
        if self.busy {
            self.busy = false;
            self.entered = false;
        }
    }
}
