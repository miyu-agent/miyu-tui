//! 第一次打开的引导（蓝图 `tui.md`「第一次打开的引导」）：欢迎 → 语言 → 图标 → 接模型 → 建人格 → 主题 → AI 怎么认识你
//! → 选预设 → 沙盒 → 好了。
//! 这里是走到哪、按键、等的回应交给哪一步；数值在 `look.rs`，动的东西在 `motion/`，画在 `ui/oobe/`，和 App 接在
//! `app/oobe.rs`。

pub mod asker;
pub mod field;
pub mod figure;
mod focus;
mod link;
pub mod look;
pub mod model;
pub mod motion;
pub mod nav;
pub mod persona;
pub mod pick;
pub mod preset;
pub mod profile;
pub mod register;
mod replies;
pub mod semantic;
pub mod sheet;
mod step;
mod texts;
pub mod theme;
pub mod wrap;

use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

pub use texts::{Semantic as SemanticTexts, Texts};

use crate::core::Refusal;
use asker::{Ask, Asker, Waiting};
use motion::{Intro, React, Slide, Stars};
use pick::Pick;
pub use step::Step;

/// 一步按了键、收到回应以后往哪走。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    /// 留在这一步。
    Stay,
    /// 下一步。
    Next,
    /// 上一步。
    Back,
}

/// 吉祥物该有的反应（第 10 条）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    /// 抬头等着。
    Wait,
    /// 连上了。
    Happy,
    /// 没连上。
    Sad,
    /// 不等了、也不高兴不失望。
    Calm,
}

/// 交给 App 办的。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// 换界面语言：语言那一步第几行（第 0 行跟随系统）。
    Language(usize),
    /// 换图标那一套。
    Icons(String),
    /// 把这段字交给 `$EDITOR`（建人格时 `Ctrl+G`）。
    Editor(String),
}

/// 好了那一屏的三样。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    /// 模型。
    pub model: String,
    /// 人格的名字。
    pub persona: String,
    /// 预设的名字。
    pub preset: String,
}

/// 整个引导。
#[derive(Debug)]
pub struct Oobe {
    /// 在第几步。
    pub step: Step,
    /// 正在滑：从哪一步滑过来。
    pub sliding: Option<(Step, Slide)>,
    /// 欢迎页的开场。
    pub intro: Intro,
    /// 好了那一屏的时间线从这一刻起。
    pub done_at: Option<Instant>,
    /// 回车走了：从这一刻起散开。
    pub leaving: Option<Instant>,
    /// 写标记的回应回来了：`Some(None)` 成了，`Some(Some(原话))` 没写成。
    pub marked: Option<Option<String>>,
    /// 星点。
    pub stars: Stars,
    /// 吉祥物的反应。
    pub react: React,
    /// 左边那一块画什么：吉祥物、头像、什么都不画（`figure.rs`）。
    pub figure: figure::Figure,
    /// 名字写到吉祥物脚下：名字、从哪一刻起写。
    pub badge: Option<(String, Instant)>,
    /// 语言那一步。
    pub language: Pick,
    /// 注册机主（还没有机主时才有）。
    pub register: register::RegisterStep,
    /// 操作方式那一步：第 0 行方向键（`arrows`）、第 1 行 Vim（`vim`）。开引导时 App 照现在的按键模式选着。
    pub keys: Pick,
    /// 图标那一步：第 0 行 `nerd`、第 1 行 `plain`。
    pub icons: Pick,
    /// 接模型。
    pub model: model::ModelStep,
    /// 建人格。
    pub persona: persona::PersonaStep,
    /// 主题。
    pub theme: theme::ThemeStep,
    /// AI 怎么认识你。
    pub profile: profile::ProfileStep,
    /// 选预设。
    pub preset: preset::PresetStep,
    pub semantic: semantic::SemanticStep,
    /// 连着核心。
    pub online: bool,
    /// 数值（`oobe.json`）。
    pub look: look::Look,
    /// 引导从这一刻起：流光照它算。
    pub born: Instant,
    asker: Asker,
    seed: u64,
}

/// 图标那一步两行各是哪一套。
pub const ICON_SETS: [&str; 2] = ["nerd", "plain"];

/// 操作方式那一步的几行，照先后：`tui.keys` 的两个值（终端软件包清单 `[settings.keys]`）。
pub const KEY_MODES: [&str; 2] = ["arrows", "vim"];

impl Oobe {
    /// 从欢迎页开场起。`language` 是语言那一步的几行和选着的，`icons` 是现在用的那一套。
    pub fn new(now: Instant, look: look::Look, language: Pick, icons: &str, online: bool) -> Self {
        let seed = crate::rng::Rng::from_clock().next();
        let icon_row = ICON_SETS.iter().position(|s| *s == icons).unwrap_or(0);
        Self {
            step: Step::Welcome,
            sliding: None,
            intro: Intro::new(now),
            done_at: None,
            leaving: None,
            marked: None,
            stars: Stars::gather(now, seed),
            react: React::default(),
            figure: figure::Figure::default(),
            badge: None,
            language,
            keys: Pick::new(KEY_MODES.iter().map(|s| (*s).to_string()).collect(), 0),
            // 两行的字照界面语言画（「看得到图标」「是方块或者乱码」），这里记哪一套。
            icons: Pick::new(
                ICON_SETS.iter().map(|s| (*s).to_string()).collect(),
                icon_row,
            ),
            register: register::RegisterStep::default(),
            model: model::ModelStep::new(look.catalog_limit),
            persona: persona::PersonaStep::default(),
            theme: theme::ThemeStep::default(),
            profile: profile::ProfileStep::default(),
            preset: preset::PresetStep::default(),
            semantic: semantic::SemanticStep::default(),
            online,
            look,
            born: now,
            asker: Asker::default(),
            seed,
        }
    }

    /// 攒着要发的请求，App 交给核心。
    pub fn take_asks(&mut self) -> Vec<Ask> {
        self.asker.take()
    }

    /// 按了一个键。`chars` 是欢迎页标题有几个字（跳过开场要用）。
    pub fn key(&mut self, key: KeyEvent, now: Instant, texts: &Texts) -> Vec<Effect> {
        self.react.poke();
        if self.leaving.is_some() {
            return Vec::new();
        }
        // 吉祥物的反应（第 10 条）：在格子里打字歪着头看。
        let typing = self.typing() && focus::is_typed(key);
        let mut effects = Vec::new();
        let moved = match self.step {
            Step::Welcome => {
                if self.intro_running(now, texts) {
                    self.intro.skip();
                    Move::Stay
                } else if key.code == KeyCode::Enter {
                    Move::Next
                } else {
                    Move::Stay
                }
            }
            Step::Language => self.pick_key(Step::Language, key, &mut effects),
            Step::Register => self.register.key(key, &mut self.asker),
            Step::Keys => self.pick_key(Step::Keys, key, &mut effects),
            Step::Icons => self.pick_key(Step::Icons, key, &mut effects),
            Step::Model => self.model.key(key, &mut self.asker, &texts.model),
            Step::Persona => self
                .persona
                .key(key, &mut self.asker, &texts.persona, &mut effects),
            Step::Theme => self.theme.key(key, &mut self.asker, &texts.theme),
            Step::Profile => {
                let words = &texts.profile;
                let combo = [
                    words.hints[profile::LANGUAGE].as_str(),
                    words.other.as_str(),
                ];
                self.profile.key(key, &mut self.asker, &mut effects, combo)
            }
            Step::Preset => self.preset.key(key, &mut self.asker, &texts.preset),
            Step::Semantic => self.semantic.key(key, &mut self.asker),
            Step::Done => {
                if !self.finale(now, texts).ready {
                    self.done_at = now.checked_sub(Duration::from_secs(3600));
                } else if key.code == KeyCode::Enter {
                    self.leave(now);
                }
                Move::Stay
            }
        };
        self.moved(moved, now);
        if typing {
            self.react.typed(now, &self.look.react);
        }
        effects
    }

    /// 粘贴：给光标那一格。
    pub fn paste(&mut self, text: &str) {
        match self.step {
            Step::Model => self.model.paste(text),
            Step::Persona => self.persona.paste(text),
            Step::Profile => self.profile.paste(text),
            Step::Preset => self.preset.paste(text),
            Step::Register => self.register.paste(text),
            _ => {}
        }
    }

    /// 编辑器退出了：写回建人格那一格。
    pub fn edited(&mut self, text: Option<String>) {
        match self.step {
            Step::Profile => self.profile.edited(text),
            _ => self.persona.edited(text),
        }
    }

    /// 等的回应回来了：是引导的交回 `true`。
    pub fn answer(
        &mut self,
        tag: u64,
        result: Result<Value, Refusal>,
        now: Instant,
        texts: &Texts,
    ) -> bool {
        let Some(waiting) = self.asker.settle(tag) else {
            return false;
        };
        self.apply(waiting, result, now, texts);
        true
    }

    /// 走完了：回车以后散开的动画播完、写标记的回应也回来了。交回写标记没成时核心的原话。
    pub fn closing(&self, now: Instant, leave: Duration) -> Option<Option<String>> {
        let left = self.leaving.is_some_and(|at| now >= at + leave);
        left.then(|| self.marked.clone()).flatten()
    }

    fn pick_key(&mut self, step: Step, key: KeyEvent, effects: &mut Vec<Effect>) -> Move {
        let pick = match step {
            Step::Language => &mut self.language,
            Step::Keys => &mut self.keys,
            _ => &mut self.icons,
        };
        if pick.key(key) {
            return Move::Stay;
        }
        match key.code {
            KeyCode::Esc => Move::Back,
            KeyCode::Enter if step == Step::Language => {
                effects.push(Effect::Language(pick.selected));
                Move::Next
            }
            KeyCode::Enter if step == Step::Keys => {
                let mode = KEY_MODES[pick.selected.min(KEY_MODES.len() - 1)];
                let params =
                    json!({"layer": "personal", "changes": [{"key": "tui.keys", "value": mode}]});
                // 写成了核心推配置变了，终端照常重读 `tui.keys` 换过去（`head_settings.rs`），不另交 App。
                self.asker.ask("config.set", params, Waiting::Keys);
                Move::Next
            }
            KeyCode::Enter => {
                let set = ICON_SETS[pick.selected.min(ICON_SETS.len() - 1)].to_string();
                let params =
                    json!({"layer": "personal", "changes": [{"key": "tui.icons", "value": set}]});
                self.asker.ask("config.set", params, Waiting::Icons);
                effects.push(Effect::Icons(set));
                Move::Next
            }
            _ => Move::Stay,
        }
    }

    /// 照这一步交回的走：滑过去、吉祥物跳一下；进了哪一步要读的发出去；反应照那一步记下的。
    fn moved(&mut self, moved: Move, now: Instant) {
        let mood = self.model.mood.take().or(self.persona.mood.take());
        match mood {
            Some(Mood::Wait) => self.react.waiting = true,
            Some(Mood::Happy) => self.react.happy(now),
            Some(Mood::Sad) => self.react.sad(now),
            Some(Mood::Calm) => self.react.waiting = false,
            None => {}
        }
        let to = match moved {
            Move::Stay => return,
            Move::Next => self.step.next(),
            Move::Back => self.step.previous(),
        };
        let to = self.skip(to, moved == Move::Back);
        if to == self.step {
            return;
        }
        self.sliding = Some((self.step, Slide::new(now, moved == Move::Back)));
        self.step = to;
        self.react.hop(now);
        self.react.waiting = false;
        match to {
            Step::Welcome => {
                self.intro.skip();
                self.stars = Stars::quiet(now, self.seed);
                self.sliding = None;
            }
            Step::Model => self.model.enter(&mut self.asker),
            Step::Persona => self.persona.enter(&mut self.asker),
            Step::Theme => {
                let follow = self.persona.colors().is_some();
                let avatar = self.persona.shows_avatar();
                let persona = self.persona.chosen_id.clone();
                self.theme.enter(&mut self.asker, (follow, avatar), persona);
            }
            Step::Profile => self.profile.enter(&mut self.asker),
            Step::Preset => {
                self.preset.enter(&mut self.asker);
                // 下一步装没装语义模型的包：先问，到那一步多半已经知道了。
                self.semantic.probe(&mut self.asker);
            }
            Step::Semantic => self.semantic.enter(&mut self.asker),
            Step::Done => {
                self.done_at = Some(now);
                self.stars = Stars::gather(now, self.seed.wrapping_add(1));
                self.sliding = None;
            }
            Step::Language | Step::Register | Step::Keys | Step::Icons => {}
        }
    }

    /// 握手说的：还没有机主的要走注册这一步；在等注册的、握手又说有机主了的往下走（第 5a 条）。
    pub fn account(&mut self, setup: bool, now: Instant) {
        if setup {
            self.register.needed = true;
        } else if self.register.registered() && self.step == Step::Register {
            self.moved(Move::Next, now);
        }
    }

    /// 要去 `to`：不用停的跨过去，照走的方向再走：没装语义模型的包跨过那一步（`semantic.rs`）；人格那一步跳过了的跨过
    /// 主题（配色跟着人格的头像，2026-10-19 项目主人）。
    fn skip(&self, to: Step, back: bool) -> Step {
        let mut to = self.semantic.skip(to, back);
        // 有机主了（本来就有、注册过了）：跨过注册（第 5a 条）。
        if to == Step::Register && !self.register.pending() {
            to = if back { to.previous() } else { to.next() };
        }
        if to == Step::Theme && self.persona.chosen_id.is_none() {
            to = if back { to.previous() } else { to.next() };
        }
        to
    }

    /// 好了那一屏回车：写标记，星点散开。
    fn leave(&mut self, now: Instant) {
        self.leaving = Some(now);
        self.stars.scatter(now);
        if self.online {
            let params =
                json!({"layer": "personal", "changes": [{"key": "ui.welcomed", "value": true}]});
            self.asker.ask("config.set", params, Waiting::Welcomed);
        } else {
            self.marked = Some(Some(String::new()));
        }
    }

    /// 欢迎页的开场还在播。
    fn intro_running(&self, now: Instant, texts: &Texts) -> bool {
        let chars = texts.welcome.title.chars().count();
        self.step == Step::Welcome && !self.intro.finished(now, &self.look.intro, chars)
    }

    /// 好了那一屏这一刻的样子（第 11 条）。
    pub fn finale(&self, now: Instant, texts: &Texts) -> motion::Finale {
        let chars = texts.done.title.chars().count();
        let start = self.done_at.unwrap_or(now);
        motion::finale(start, now, &self.look, chars)
    }

    /// 引导里写成默认的人格、预设的编号：紧接着的新会话照它们开，不再问（第 11 条）。
    pub fn picks(&self) -> (Option<String>, Option<String>) {
        (
            self.persona.chosen_id.clone(),
            self.preset.chosen_id.clone(),
        )
    }

    /// 左边那一块这一刻该画什么（第 11a 条，2026-10-15 项目主人）：人格那一步以前画吉祥物；从人格那一步起，这一位有头像
    /// （`avatar`：画得出来的头像文件）的画头像；主题那一步选着吉祥物不显示的（在这一步、写成了的）什么都不画。
    pub fn wanted_figure(&self, avatar: bool) -> figure::Shown {
        let early = matches!(
            self.step,
            Step::Welcome | Step::Language | Step::Keys | Step::Icons | Step::Model
        );
        if early {
            return figure::Shown::Mascot;
        }
        if avatar {
            return figure::Shown::Avatar;
        }
        let off = self.theme.mascot_off && (self.step == Step::Theme || self.theme.saved);
        if off {
            figure::Shown::Nothing
        } else {
            figure::Shown::Mascot
        }
    }

    /// 主题那一步选着的样子（第 23a 条）：在这一步、读到了配置的，或者写成了的（引导关上以前一直照它）交回配色、明暗、
    /// 铺底色、挑过的主色；别的时候是 `None`，照配置项。
    pub fn theme_preview(&self) -> Option<theme::Preview> {
        (self.step == Step::Theme || self.theme.saved)
            .then(|| self.theme.preview())
            .flatten()
    }

    /// 好了那一屏三样。
    pub fn summary(&self) -> Summary {
        Summary {
            model: self.model.chosen.clone().unwrap_or_default(),
            persona: self.persona.chosen.clone().unwrap_or_default(),
            preset: self.preset.chosen.clone().unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests;
