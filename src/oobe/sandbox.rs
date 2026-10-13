//! 沙盒那一步（蓝图 `tui.md`「第一次打开的引导」第 26a 条，2026-10-13 项目主人定）：新会话怎么开局。「开」（默认）是工作区
//! 级别，命令在沙盒里跑、工作区以外的改动先问；「关」是完全放开。写个人设置的 `permission.level`（核心 8-32：`workspace`、
//! `full`，新会话生效）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::asker::{Asker, Waiting};
use super::pick::Pick;
use super::{Move, Texts};
use crate::core::Refusal;

/// 两行各写哪一档：第 0 行开（工作区），第 1 行关（完全放开）。
pub const LEVELS: [&str; 2] = ["workspace", "full"];

/// 这一步。
#[derive(Debug)]
pub struct SandboxStep {
    /// 选着开还是关。
    pub pick: Pick,
    /// 在写、在读：按键先不认（`Esc` 照常回上一步）。
    pub busy: bool,
    /// 写不成：核心的原话。
    pub error: Option<String>,
    entered: bool,
}

impl Default for SandboxStep {
    fn default() -> Self {
        Self {
            pick: Pick::new(LEVELS.iter().map(|l| (*l).to_string()).collect(), 0),
            busy: false,
            error: None,
            entered: false,
        }
    }
}

impl SandboxStep {
    /// 进这一步：第一次进的先读现在写的是哪一档（重走引导时照它停），读不到的停在「开」。
    pub fn enter(&mut self, asker: &mut Asker) {
        if self.entered {
            return;
        }
        self.entered = true;
        self.busy = true;
        asker.ask(
            "config.get",
            json!({"keys": ["permission.level"]}),
            Waiting::SandboxRead,
        );
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
            let level = LEVELS[self.pick.selected.min(LEVELS.len() - 1)];
            let params = json!({"layer": "personal", "changes": [{"key": "permission.level", "value": level}]});
            self.error = None;
            self.busy = true;
            asker.ask("config.set", params, Waiting::SandboxSave);
        }
        Move::Stay
    }

    /// 回应：读到的照它停；写成了下一步，写不成的写原话、留在这一步。
    pub fn answer(
        &mut self,
        waiting: &Waiting,
        result: Result<Value, Refusal>,
        _texts: &Texts,
    ) -> Move {
        self.busy = false;
        match waiting {
            Waiting::SandboxRead => {
                let level = result.ok().and_then(|g| {
                    g["items"]["permission.level"]["value"]
                        .as_str()
                        .map(str::to_string)
                });
                if let Some(at) = level.and_then(|l| LEVELS.iter().position(|v| *v == l)) {
                    self.pick.selected = at;
                }
                Move::Stay
            }
            Waiting::SandboxSave => match result {
                Ok(_) => Move::Next,
                Err(refusal) => {
                    self.error = Some(crate::settings::pages::target::said(&refusal));
                    Move::Stay
                }
            },
            _ => Move::Stay,
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
