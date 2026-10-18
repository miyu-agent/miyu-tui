//! 引导连上、断开（review B6，2026-10-11）：断开时等着的都不会回来了。试连接、保存、写标记这些动作照被拒办，各步退回去、
//! 写断开的原因；读列表这些忘掉，忙着的复位；又连上了，正在这一步的重新要。原来只是忘掉：在试、在存的不认任何键，人格、
//! 预设两步一直忙着，最后一屏回车以后关不掉，只能 Ctrl+C 退出。

use std::time::Instant;

use serde_json::Value;

use super::{Oobe, Step, Texts};
use crate::core::Refusal;

impl Oobe {
    /// 又连上了：正在的这一步该读的还没读到的重新要。
    pub fn online(&mut self) {
        self.online = true;
        match self.step {
            Step::Model => self.model.enter(&mut self.asker),
            Step::Persona => self.persona.enter(&mut self.asker),
            Step::Preset => self.preset.enter(&mut self.asker),
            _ => {}
        }
    }

    /// 断开了：动作照被拒办（原因写 `lost`），读的忘掉、忙着的复位（下次进这一步、重连时再要）。
    pub fn offline(&mut self, lost: &str, now: Instant, texts: &Texts) {
        self.online = false;
        let refusal = Refusal {
            reason: None,
            message: lost.to_string(),
            data: Value::Null,
        };
        for waiting in self.asker.forget() {
            if !waiting.reads() {
                self.apply(waiting, Err(refusal.clone()), now, texts);
            }
        }
        self.persona.lost();
        self.preset.lost();
        self.semantic.lost();
        self.theme.lost();
        self.profile.lost();
    }
}
