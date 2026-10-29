//! 主题那一步（蓝图 `tui.md`「第一次打开的引导」第 23a 条，2026-10-13 项目主人定，版面 2026-10-14 定）：明暗、配色两行，
//! 在一行上按 `Enter` 开选择窗（和配置页选值的同一个、同一套键，2026-10-14 项目主人：以后主题多了、有别人做的，左右切不
//! 好用），上下挪的时候整个界面当场换成挪到的样子（App 每一帧照 [`ThemeStep::preview`] 挑，`app/theme.rs`），`Enter` 选好，
//! `Esc` 换回开窗以前的。「下一步 →」写系统配置的 `tui.theme_mode`、`tui.theme`，`Esc` 回上一步、换回原来的样子。配色没
//! 写过、人格那一步有头像（或写了主题色）的，进来停在「跟随头像」。人格那一步的这一位没头像的，多一格「吉祥物」显示、
//! 不显示，一格同时写 `tui.mascot_home`、`tui.mascot_sidebar`（2026-10-15 项目主人定：有头像的头像替掉吉祥物，不列）。
//! 跟随头像、跟随壁纸、有候选时多一格「主色」，开色块列表挑（跟随壁纸的写 `tui.theme_seed`，跟随头像的写回这个人格）；
//! 再一格「铺底色」开、关，写 `tui.theme_background`（2026-10-15 项目主人：配色方案和主色，铺底色也缺了）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::Move;
use super::asker::{Asker, Waiting};
use super::nav::{self, Nav};
use super::texts::Theme as Words;
use crate::core::Refusal;
use crate::settings::pages::choose::{Choice, Choose, Pressed};
use crate::theme::choice::{COLORS, GROUND_KEY, Mode, SEED_KEY, SEEDED};

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
    /// 主色（跟随头像、跟随壁纸、有候选时才有）。
    Seed,
    /// 铺底色：开、关。
    Ground,
    /// 吉祥物：显示、不显示（人格没头像时才有）。
    Mascot,
    /// 「下一步 →」。
    Next,
}

/// 吉祥物那一格写的两项：首页、侧边栏一起。
const MASCOT_KEYS: [&str; 2] = ["tui.mascot_home", "tui.mascot_sidebar"];
/// 选着的样子：App 照它盖在配置项上（`app/theme.rs`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    pub colors: String,
    pub mode: Mode,
    pub ground: bool,
    /// 这一回挑的主色；没挑的照配置、人格的。
    pub seed: Option<u32>,
}

/// 这一步。
#[derive(Debug)]
pub struct ThemeStep {
    /// 光标停在哪一行。
    pub slot: Slot,
    /// 明暗选着第几档（[`MODES`]）。
    pub mode: usize,
    /// 配色选着第几种（`choice::COLORS`）。
    pub colors: usize,
    /// 吉祥物选着「不显示」（配置项默认显示）。
    pub mascot_off: bool,
    /// 铺底色（配置项默认开）。
    pub ground: bool,
    /// 能当主色的候选（好的在前）、正用着的那个：跟随头像、跟随壁纸时 App 每一拍设（`app/theme.rs`）。
    pub seeds: Vec<u32>,
    pub seed: Option<u32>,
    /// 这一回挑的主色：当场换样子，「下一步」时写。
    pub seed_pick: Option<u32>,
    /// 开主色的窗以前挑的：`Esc` 换回它。
    seed_before: Option<u32>,
    /// 跟随头像的主色写给哪个人格：人格那一步存好的编号。
    persona: Option<String>,
    /// 配置写成了，还要写人格的主色。
    seed_pending: bool,
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

impl Default for ThemeStep {
    fn default() -> Self {
        Self {
            slot: Slot::default(),
            mode: 0,
            colors: 0,
            mascot_off: false,
            ground: true,
            seeds: Vec::new(),
            seed: None,
            seed_pick: None,
            seed_before: None,
            persona: None,
            seed_pending: false,
            busy: false,
            error: None,
            saved: false,
            read: false,
            unset: false,
            turned: false,
            entered: false,
            picking: None,
            before: 0,
            wallpaper: false,
        }
    }
}

impl ThemeStep {
    /// 进这一步：第一次进的先读现在写的（重走引导时照它停）。`follow` 是人格那一步有没有能取色的（头像、主题色）：配色没
    /// 写过、没换过的停在「跟随头像」。「形象」那一格有没有头像都列（2026-10-29 项目主人：头像也归它管）。`persona`
    /// 是人格那一步存好的编号：跟随头像挑的主色写给它。
    pub fn enter(&mut self, asker: &mut Asker, follow: bool, persona: Option<String>) {
        self.persona = persona;
        if !self.entered {
            self.entered = true;
            self.busy = true;
            let keys = json!({"keys": [
                "tui.theme", "tui.theme_mode", MASCOT_KEYS[0], MASCOT_KEYS[1], GROUND_KEY
            ]});
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

    /// 按了一个键：开着选择窗的归它（挪了当场换样子，`Esc` 换回开窗以前的）；不然上下换行，几格那一行 `Enter` 开选择窗，
    /// 「下一步」那一行 `Enter` 写，`Esc` 回上一步。
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
                    if self.slot == Slot::Seed {
                        self.seed_pick = self.seed_before;
                    } else {
                        let before = self.before;
                        self.set(before);
                    }
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
        let slots = self.slots();
        let at = slots.iter().position(|s| *s == self.slot).unwrap_or(0);
        match nav::plain(key) {
            Some(Nav::Up) => self.slot = slots[at.saturating_sub(1)],
            Some(Nav::Down) => self.slot = slots[(at + 1).min(slots.len() - 1)],
            None if key.code == KeyCode::Enter => match self.slot {
                Slot::Next => self.save(asker),
                _ => self.open(words),
            },
            _ => {}
        }
        Move::Stay
    }

    /// 配色是跟随头像、跟随壁纸、终端配色，有主色的候选：列「主色」那一格。
    pub fn seeding(&self) -> bool {
        COLORS.get(self.colors).is_some_and(|c| SEEDED.contains(c)) && !self.seeds.is_empty()
    }

    /// 从上往下有哪几行：没候选的没有主色那一格，配色是终端配色的没有铺底色那一格（2026-10-15 项目主人定：永远露出终端
    /// 自己的底，换走了再出来、值照原来的），人格有头像的没有吉祥物那一格。
    pub fn slots(&self) -> Vec<Slot> {
        [
            Slot::Mode,
            Slot::Colors,
            Slot::Seed,
            Slot::Ground,
            Slot::Mascot,
            Slot::Next,
        ]
        .into_iter()
        .filter(|s| match s {
            Slot::Seed => self.seeding(),
            Slot::Ground => COLORS.get(self.colors) != Some(&"terminal"),
            _ => true,
        })
        .collect()
    }

    /// 主色那一格画的：挑过的，没挑的照正用着的。
    pub fn shown_seed(&self) -> Option<u32> {
        self.seed_pick
            .filter(|s| self.seeds.contains(s))
            .or(self.seed)
    }

    /// 光标那一行开选择窗：一个选项一行，停在选着的那个上。
    fn open(&mut self, words: &Words) {
        let switch = |names: &[String; 2], on: bool| -> (Vec<Choice>, Value) {
            let choices = names
                .iter()
                .zip([true, false])
                .map(|(name, value)| Choice {
                    value: json!(value),
                    label: name.clone(),
                    note: None,
                    off: None,
                })
                .collect();
            (choices, json!(on))
        };
        match self.slot {
            Slot::Mascot => {
                let (choices, current) = switch(&words.mascot, !self.mascot_off);
                self.before = usize::from(self.mascot_off);
                self.picking = Some(Choose::open(
                    MASCOT_KEYS[0],
                    &words.rows[2],
                    choices,
                    &current,
                ));
                return;
            }
            Slot::Ground => {
                let (choices, current) = switch(&words.ground, self.ground);
                self.before = usize::from(!self.ground);
                self.picking = Some(Choose::open(GROUND_KEY, &words.rows[4], choices, &current));
                return;
            }
            Slot::Seed => {
                let choices = self
                    .seeds
                    .iter()
                    .enumerate()
                    .map(|(i, seed)| Choice {
                        value: Value::from(format!("#{seed:06x}")),
                        label: format!("#{seed:06x}"),
                        note: (i == 0).then(|| words.seed_default.clone()),
                        off: None,
                    })
                    .collect();
                let current = self
                    .shown_seed()
                    .map_or(Value::Null, |s| Value::from(format!("#{s:06x}")));
                self.seed_before = self.seed_pick;
                let mut choose = Choose::open(SEED_KEY, &words.rows[3], choices, &current);
                choose.swatches = true;
                self.picking = Some(choose);
                return;
            }
            Slot::Mode | Slot::Colors | Slot::Next => {}
        }
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
            _ => return,
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
        match self.slot {
            Slot::Mascot | Slot::Ground => {
                if let Some(on) = value.and_then(Value::as_bool) {
                    self.set(usize::from(!on));
                }
                return;
            }
            Slot::Seed => {
                let hex = value
                    .and_then(Value::as_str)
                    .and_then(|v| v.strip_prefix('#'));
                if let Some(seed) = hex.and_then(|h| u32::from_str_radix(h, 16).ok()) {
                    self.seed_pick = Some(seed);
                    self.saved = false;
                }
                return;
            }
            Slot::Mode | Slot::Colors | Slot::Next => {}
        }
        let Some(value) = value.and_then(Value::as_str) else {
            return;
        };
        let at = match self.slot {
            Slot::Mode => MODE_NAMES.iter().position(|v| *v == value),
            Slot::Colors => COLORS.iter().position(|v| *v == value),
            _ => None,
        };
        if let Some(at) = at {
            self.set(at);
        }
    }

    /// 光标那一行选成第 `at` 个：当场换样子（吉祥物、铺底色是第 0 个开、第 1 个关）。换了配色，挑过的主色不算了。
    fn set(&mut self, at: usize) {
        match self.slot {
            Slot::Mode => self.mode = at.min(MODES.len() - 1),
            Slot::Mascot => self.mascot_off = at == 1,
            Slot::Ground => self.ground = at == 0,
            Slot::Colors => {
                let colors = at.min(COLORS.len() - 1);
                if colors != self.colors {
                    self.seed_pick = None;
                }
                self.colors = colors;
                self.turned = true;
            }
            Slot::Seed | Slot::Next => return,
        }
        self.saved = false;
    }

    fn save(&mut self, asker: &mut Asker) {
        let colors = COLORS[self.colors];
        let mut changes = vec![
            json!({"key": "tui.theme_mode", "value": MODE_NAMES[self.mode]}),
            json!({"key": "tui.theme", "value": colors}),
            json!({"key": GROUND_KEY, "value": self.ground}),
        ];
        changes.extend(MASCOT_KEYS.map(|key| json!({"key": key, "value": !self.mascot_off})));
        // 挑过主色的：跟随壁纸、终端配色的写系统配置，跟随头像的写完配置再写人格（`seed_pending`）。
        let pick = self.seed_pick.filter(|_| self.seeding());
        if let (Some(seed), "wallpaper" | "terminal") = (pick, colors) {
            changes.push(json!({"key": SEED_KEY, "value": format!("#{seed:06x}")}));
        }
        self.seed_pending = pick.is_some() && colors == "persona" && self.persona.is_some();
        let params = json!({"changes": changes});
        self.error = None;
        self.busy = true;
        asker.ask("config.set", params, Waiting::ThemeSave);
    }

    /// 回应：读到的照它停；写成了下一步（还要写人格的主色的接着写），写不成的写原话、留在这一步。
    pub fn answer(
        &mut self,
        waiting: &Waiting,
        result: Result<Value, Refusal>,
        (follow, asker): (bool, &mut Asker),
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
                // 吉祥物照首页那一项停：人设过关的停在不显示；没人设过的照引导自己的默认显示（出厂值 2026-10-26 起是关，
                // 项目主人：引导里的照旧）。铺底色没有的照默认开。
                let mascot = &items[MASCOT_KEYS[0]];
                let set = mascot["origin"]["layer"]
                    .as_str()
                    .is_some_and(|layer| layer != "default");
                self.mascot_off = set && mascot["value"].as_bool() == Some(false);
                self.ground = items[GROUND_KEY]["value"].as_bool() != Some(false);
                let layer = items["tui.theme"]["origin"]["layer"].as_str();
                self.unset = layer.is_none_or(|l| l == "default");
                self.read = true;
                self.settle(follow);
                Move::Stay
            }
            Waiting::ThemeSave => match result {
                Ok(_) if self.seed_pending => {
                    self.seed_pending = false;
                    self.saved = true;
                    let seed = self.seed_pick.map(|s| format!("#{s:06x}"));
                    let params = json!({"persona": self.persona, "seed": seed});
                    self.busy = true;
                    asker.ask("persona.set", params, Waiting::ThemeSeed);
                    Move::Stay
                }
                Ok(_) => {
                    self.saved = true;
                    Move::Next
                }
                Err(refusal) => {
                    self.error = Some(crate::settings::pages::target::said(&refusal));
                    Move::Stay
                }
            },
            Waiting::ThemeSeed => match result {
                Ok(_) => Move::Next,
                Err(refusal) => {
                    self.error = Some(crate::settings::pages::target::said(&refusal));
                    Move::Stay
                }
            },
            _ => Move::Stay,
        }
    }

    /// 选着的样子：配色、明暗、铺底色、挑过的主色；还没读到现在的配置的是 `None`（照原来的样子）。
    pub fn preview(&self) -> Option<Preview> {
        self.read.then(|| Preview {
            colors: COLORS[self.colors].to_string(),
            mode: MODES[self.mode],
            ground: self.ground,
            seed: self.seed_pick,
        })
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
