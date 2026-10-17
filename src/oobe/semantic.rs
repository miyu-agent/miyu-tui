//! 内置语义模型那一步（蓝图 `tui.md`「第一次打开的引导」第 26b 条，2026-10-18 项目主人定）：沙盒后面、好了前面，问开不开
//! 装的语义模型包（embed）。只在装了的时候出现：`config.schema` 里 `models.embedding` 的 `local` 那一档不带
//! `available: false`。「是」写个人设置的 `models.embedding = "local"`，「否」写 `"off"`（只用关键词）；没写时默认就是
//! `local`。没装的跨过去，往回走也跨过去。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::asker::{Asker, Waiting};
use super::pick::Pick;
use super::{Move, Step};
use crate::core::Refusal;

/// 配置项。
const KEY: &str = "models.embedding";
/// 两行各写哪一档：第 0 行是（本机的语义模型），第 1 行否（只用关键词）。
pub const VALUES: [&str; 2] = ["local", "off"];

/// 这一步。
#[derive(Debug)]
pub struct SemanticStep {
    /// 选着是还是否。
    pub pick: Pick,
    /// 在写、在读：按键先不认（`Esc` 照常回上一步）。
    pub busy: bool,
    /// 写不成：核心的原话。
    pub error: Option<String>,
    /// 装没装：问到以前是 `None`。
    pub installed: Option<bool>,
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
            current: 0,
            asked: false,
            entered: false,
        }
    }
}

impl SemanticStep {
    /// 先问装没装（进沙盒那一步时就问，到这一步多半已经知道了）。
    pub fn probe(&mut self, asker: &mut Asker) {
        if !self.asked {
            self.asked = true;
            asker.ask("config.schema", json!({}), Waiting::SemanticSchema);
        }
    }

    /// 要去 `to`：没装的跨过这一步，照走的方向再走一格。
    pub fn skip(&self, to: Step, back: bool) -> Step {
        if to == Step::Semantic && self.installed == Some(false) {
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

    /// 按了一个键：上下选，`Enter` 写，`Esc` 回上一步。
    pub fn key(&mut self, key: KeyEvent, asker: &mut Asker) -> Move {
        if key.code == KeyCode::Esc {
            return Move::Back;
        }
        if self.busy {
            return Move::Stay;
        }
        if self.pick.key(key) {
            return Move::Stay;
        }
        if key.code == KeyCode::Enter {
            if self.pick.selected == self.current {
                return Move::Next;
            }
            let value = VALUES[self.pick.selected.min(VALUES.len() - 1)];
            let params = json!({"layer": "personal", "changes": [{"key": KEY, "value": value}]});
            self.error = None;
            self.busy = true;
            asker.ask("config.set", params, Waiting::SemanticSave);
        }
        Move::Stay
    }

    /// 回应：装没装记下（正停在这一步、又没装的往下走）；读到的照它停；写成了下一步，写不成的写原话。
    pub fn answer(
        &mut self,
        waiting: &Waiting,
        result: Result<Value, Refusal>,
        here: bool,
    ) -> Move {
        match waiting {
            Waiting::SemanticSchema => {
                let installed = result.ok().is_some_and(|got| installed(&got));
                self.installed = Some(installed);
                if here && !installed {
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
        if self.installed.is_none() {
            self.asked = false;
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
