//! 主题那一步（蓝图 `tui.md`「第一次打开的引导」第 23a 条，2026-10-13 项目主人定，版面 2026-10-14 定）：明暗、配色两行
//! 左右切换，选的时候整个界面当场换成选的样子（App 每一帧照 [`ThemeStep::preview`] 挑，`app/theme.rs`）。`Enter` 写个人
//! 设置的 `tui.theme_mode`、`tui.theme`，`Esc` 回上一步、换回原来的样子。配色没写过、人格那一步有头像（或写了主题色）的，
//! 进来停在「跟随头像」。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::Move;
use super::asker::{Asker, Waiting};
use super::nav::{self, Nav};
use crate::core::Refusal;
use crate::theme::choice::{COLORS, Mode};

/// 明暗三档，照这个先后左右换；配置里的写法同下标。
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
    /// 这一回左右换过配色：不再替人停到「跟随头像」。
    turned: bool,
    entered: bool,
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

    /// 按了一个键：上下换行，左右换值（当场换样子），`Enter` 写，`Esc` 回上一步。
    pub fn key(&mut self, key: KeyEvent, asker: &mut Asker) -> Move {
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
            Some(Nav::Left) => self.turn(MODES.len() - 1),
            Some(Nav::Right) => self.turn(1),
            None if key.code == KeyCode::Enter => self.save(asker),
            None => {}
        }
        Move::Stay
    }

    /// 光标那一行往右挪 `by` 个（左挪是挪「个数 - 1」个），到头绕回来。
    fn turn(&mut self, by: usize) {
        match self.slot {
            Slot::Mode => self.mode = (self.mode + by) % MODES.len(),
            Slot::Colors => {
                self.colors = (self.colors + by) % COLORS.len();
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
