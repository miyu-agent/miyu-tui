//! 等的回应交给哪一步（从 `mod.rs` 挪出来，2026-10-13：加步骤以后过了 500 行）。

use std::time::Instant;

use serde_json::Value;

use super::asker::Waiting;
use super::{Move, Oobe, Texts};
use crate::core::Refusal;

impl Oobe {
    /// 交给等着它的那一步：写标记的记下成没成，接模型、人格、预设的归各自那一步；走到下一步、回上一步照它交回的。
    pub(super) fn apply(
        &mut self,
        waiting: Waiting,
        result: Result<Value, Refusal>,
        now: Instant,
        texts: &Texts,
    ) {
        let moved = match &waiting {
            Waiting::Icons => Move::Stay,
            Waiting::Welcomed => {
                self.marked = Some(
                    result
                        .err()
                        .map(|r| crate::settings::pages::target::said(&r)),
                );
                Move::Stay
            }
            Waiting::Models
            | Waiting::Detect
            | Waiting::Catalog
            | Waiting::AllProviders
            | Waiting::Pools
            | Waiting::Test
            | Waiting::Secret
            | Waiting::SaveModel => {
                self.model
                    .answer(&waiting, result, &mut self.asker, &texts.model)
            }
            Waiting::PersonaDefault
            | Waiting::PersonaGet(_)
            | Waiting::PersonaRead(..)
            | Waiting::PersonaSave
            | Waiting::AvatarPut
            | Waiting::PersonaPick(_) => {
                self.persona
                    .answer(&waiting, result, &mut self.asker, &texts.persona)
            }
            Waiting::ProfileRead | Waiting::ProfileSave => self.profile.answer(&waiting, result),
            Waiting::SandboxRead | Waiting::SandboxSave => {
                self.sandbox.answer(&waiting, result, texts)
            }
            Waiting::Presets
            | Waiting::PresetDefault
            | Waiting::PresetGet(_)
            | Waiting::PresetSave
            | Waiting::PresetPick(_) => {
                self.preset
                    .answer(&waiting, result, &mut self.asker, &texts.preset)
            }
        };
        // 人格建好了：名字写到吉祥物脚下（第 10 条）。
        if let Some(name) = self.persona.badge.take() {
            self.badge = Some((name, now));
        }
        self.moved(moved, now);
    }
}
