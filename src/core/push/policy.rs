//! 换了策略（`session.policy_changed`）：权限级别、人格、预设、思考强度、上下文上限、模型（核心 8-10、P-13、SM-1、CTX-1）。

use serde_json::Value;

use super::Push;
use crate::core::kinds::Level;

/// 读成几样：认不出的级别（新版本才有的）不画，照旧显示上一次的。
pub(super) fn read(body: &Value, out: &mut Vec<Push>) {
    let level = body["permission"]["level"].as_str().unwrap_or_default();
    if let Some(level) = Level::parse(level) {
        let read_only = body["permission"]["read_only"] == true;
        out.push(Push::Policy { level, read_only });
    }
    // 中途换人格、预设（核心 P-13）：之后核心马上开一轮压缩，照手动压缩画。
    if let Some(persona) = body["persona"].as_str() {
        out.push(Push::PersonaSet(persona.to_string()));
    }
    if let Some(preset) = body["preset"].as_str() {
        out.push(Push::PresetSet(preset.to_string()));
    }
    if let Some(effort) = body.get("effort") {
        out.push(Push::EffortSet(
            effort["level"].as_str().map(str::to_string),
        ));
    }
    if let Some(cap) = crate::core::limits::cap_changed(body) {
        out.push(Push::ContextSet(cap));
    }
    if let Some(reference) = body["model"].as_str() {
        out.push(Push::ModelSet {
            reference: reference.to_string(),
            replaced: body["replaced"].as_str().map(str::to_string),
        });
    }
}
