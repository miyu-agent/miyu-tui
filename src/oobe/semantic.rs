//! 内置语义模型那一步（蓝图 `tui.md`「第一次打开的引导」第 26b 条，2026-10-18 项目主人定）：预设后面、好了前面，问开不开
//! 语义模型的包（embed）。装没装照 `config.schema` 里 `models.embedding` 的 `local` 那一档带不带 `available: false`。
//! 「是」写系统配置的 `models.embedding = "local"`，「否」写 `"off"`（只用关键词）；没写时默认就是 `local`。
//!
//! 没装的（2026-10-19 核心 G-8、项目主人定）：看一眼软件源里有没有、要下多大，有的照样出这一屏，选「是」当场下、画进度条，
//! 装不上的照原话写、给「重试」「跳过」（`download.rs`）；软件源里没有、没有这台机器能用的，往前、往回都跨过去。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::asker::{Asker, Waiting};
use super::pick::Pick;
use super::{Move, Step};
use crate::core::Refusal;

pub mod download;

use download::{Offer, Phase};

/// 配置项。
const KEY: &str = "models.embedding";
/// 两行各写哪一档：第 0 行是（本机的语义模型），第 1 行否（只用关键词）。
pub const VALUES: [&str; 2] = ["local", "off"];

/// 这一步。
#[derive(Debug)]
pub struct SemanticStep {
    /// 选着哪一行：「是」「否」，装不上以后是「重试」「跳过」。
    pub pick: Pick,
    /// 在写、在读：按键先不认（`Esc` 照常回上一步）。
    pub busy: bool,
    /// 写不成：核心的原话。
    pub error: Option<String>,
    /// 装没装：问到以前是 `None`。
    pub installed: Option<bool>,
    /// 没装的看一眼的结果：还没看到的是 `None`。
    pub offer: Option<Offer>,
    /// 下载到哪了。
    pub phase: Phase,
    /// 现在写的是哪一行（没写过的是「是」）：选的和它一样不写（同网页）。
    current: usize,
    asked: bool,
    entered: bool,
}

impl Default for SemanticStep {
    fn default() -> Self {
        Self {
            pick: Pick::new(VALUES.iter().map(|v| (*v).to_string()).collect(), 0),
            busy: false,
            error: None,
            installed: None,
            offer: None,
            phase: Phase::Choose,
            current: 0,
            asked: false,
            entered: false,
        }
    }
}

impl SemanticStep {
    /// 先问装没装（进预设那一步时就问，到这一步多半已经知道了）；没装的接着看一眼软件源里的。
    pub fn probe(&mut self, asker: &mut Asker) {
        if !self.asked {
            self.asked = true;
            asker.ask("config.schema", json!({}), Waiting::SemanticSchema);
        }
    }

    /// 这一屏不出：没装、软件源里也没有这台机器能用的。
    fn hidden(&self) -> bool {
        self.installed == Some(false) && self.offer == Some(Offer::Unavailable)
    }

    /// 要去 `to`：不出的跨过这一步，照走的方向再走一格。
    pub fn skip(&self, to: Step, back: bool) -> Step {
        if to == Step::Semantic && self.hidden() {
            return if back { to.previous() } else { to.next() };
        }
        to
    }

    /// 进这一步：第一次进的读现在写的是哪一档（重走引导时照它停），没写的停在「是」。
    pub fn enter(&mut self, asker: &mut Asker) {
        self.probe(asker);
        if self.entered {
            return;
        }
        self.entered = true;
        self.busy = true;
        asker.ask("config.get", json!({"keys": [KEY]}), Waiting::SemanticRead);
    }

    /// 正在下：按键都不认。
    pub fn installing(&self) -> bool {
        matches!(self.phase, Phase::Installing { .. })
    }

    /// 按了一个键：上下选，`Enter` 照这一行办，`Esc` 回上一步（下着的时候不认）。
    pub fn key(&mut self, key: KeyEvent, asker: &mut Asker) -> Move {
        if self.installing() {
            return Move::Stay;
        }
        if key.code == KeyCode::Esc {
            return Move::Back;
        }
        if self.busy || self.pick.key(key) || key.code != KeyCode::Enter {
            return Move::Stay;
        }
        if matches!(self.phase, Phase::Failed { .. }) {
            // 「重试」「跳过」：跳过的不写设置。
            if self.pick.selected == 1 {
                return Move::Next;
            }
            self.start(asker);
            return Move::Stay;
        }
        if self.pick.selected == 0 && self.installed == Some(false) {
            self.start(asker);
            return Move::Stay;
        }
        self.choose(self.pick.selected, asker)
    }

    /// 开始下：两行换成进度条。
    fn start(&mut self, asker: &mut Asker) {
        self.phase = Phase::Installing { done: 0, total: 0 };
        self.error = None;
        asker.ask(
            "package.install",
            download::install(),
            Waiting::SemanticInstall,
        );
    }

    /// 定下第 `at` 行：和现在写的一样的不写、直接到好了，不一样的写配置（不写层，「设置只剩一份」第 1 条）。
    fn choose(&mut self, at: usize, asker: &mut Asker) -> Move {
        if at == self.current {
            return Move::Next;
        }
        let value = VALUES[at.min(VALUES.len() - 1)];
        let params = json!({"changes": [{"key": KEY, "value": value}]});
        self.error = None;
        self.busy = true;
        asker.ask("config.set", params, Waiting::SemanticSave);
        Move::Stay
    }

    /// 推来的下载进度：只认语义模型的包、正在下的时候。
    pub fn progress(&mut self, package: &str, done: u64, total: u64) {
        if package == download::PACKAGE && self.installing() {
            self.phase = Phase::Installing { done, total };
        }
    }

    /// 回应：装没装记下（没装的接着看一眼；正停在这一步、又不出的往下走）；读到的照它停；写成了、装成了下一步，写不成的、
    /// 装不上的写原话。
    pub fn answer(
        &mut self,
        waiting: &Waiting,
        result: Result<Value, Refusal>,
        here: bool,
        asker: &mut Asker,
    ) -> Move {
        match waiting {
            Waiting::SemanticSchema => {
                let installed = result.ok().is_some_and(|got| installed(&got));
                self.installed = Some(installed);
                if !installed {
                    asker.ask(
                        "package.install",
                        download::preview(),
                        Waiting::SemanticPreview,
                    );
                }
                Move::Stay
            }
            Waiting::SemanticPreview => {
                self.offer = Some(Offer::read(result));
                if here && self.hidden() {
                    return Move::Next;
                }
                Move::Stay
            }
            Waiting::SemanticRead => {
                self.busy = false;
                let value = result
                    .ok()
                    .and_then(|g| g["items"][KEY]["value"].as_str().map(str::to_string));
                if let Some(at) = value.and_then(|v| VALUES.iter().position(|x| *x == v)) {
                    self.pick.selected = at;
                    self.current = at;
                }
                Move::Stay
            }
            Waiting::SemanticInstall => match result {
                Ok(_) => {
                    // 装上了：写的是「否」的改成「是」，本来就是的直接到好了。
                    self.installed = Some(true);
                    self.phase = Phase::Choose;
                    self.pick.selected = 0;
                    self.choose(0, asker)
                }
                Err(refusal) => {
                    self.phase = download::failed(&refusal);
                    self.pick.selected = 0;
                    Move::Stay
                }
            },
            Waiting::SemanticSave => {
                self.busy = false;
                match result {
                    Ok(_) => {
                        self.current = self.pick.selected;
                        Move::Next
                    }
                    Err(refusal) => {
                        self.error = Some(crate::settings::pages::target::said(&refusal));
                        Move::Stay
                    }
                }
            }
            _ => Move::Stay,
        }
    }

    /// 断开了，读的没回来：不再忙，下次进这一步再读。
    pub fn lost(&mut self) {
        if self.busy {
            self.busy = false;
            self.entered = false;
        }
        if self.installed.is_none() || (self.installed == Some(false) && self.offer.is_none()) {
            self.asked = false;
            self.installed = None;
        }
    }
}

/// `config.schema` 交回的：`models.embedding` 有 `local` 这一档、没写 `available: false`，就是装了语义模型的包。
pub fn installed(schema: &Value) -> bool {
    schema["items"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|item| item["key"] == KEY)
        .and_then(|item| item["options"].as_array())
        .into_iter()
        .flatten()
        .any(|o| o["value"] == "local" && o["available"] != false)
}

#[cfg(test)]
mod tests;
