//! 跟随头像的「主色」写在人格上（蓝图「配置页」，2026-10-14）：清掉人格的主题色、写好以后的回应。从 `keys.rs` 挪出来（那个
//! 文件到了 500 行）。

use serde_json::{Value, json};

use crate::settings::{Settings, Texts, Tone, Waiting};

impl Settings {
    /// 清掉人格的主题色（`persona.set` 的 `seed: {"unset": true}`）。
    pub(in crate::settings) fn clear_persona_seed(&mut self, persona: &str) {
        let params = json!({"persona": persona, "seed": {"unset": true}});
        self.ask("persona.set", params, Waiting::PersonaSeed);
    }

    /// 人格的主色写好了：说一声、叫 App 重读人格列表照新的换颜色；没写成的写原话。
    pub(in crate::settings) fn persona_seed_saved(
        &mut self,
        result: Result<Value, crate::core::Refusal>,
        texts: &Texts,
    ) {
        match result {
            Ok(_) => {
                self.say(texts.status("saved"), Tone::Good);
                self.personas_changed = true;
            }
            Err(refusal) => self.say(super::target::said(&refusal), Tone::Bad),
        }
    }
}
