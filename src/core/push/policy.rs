//! 换了策略（`session.policy_changed`）：权限级别、人格、预设、思考强度、上下文上限、压缩触发线、模型（核心 8-10、P-13、SM-1、
//! CTX-1、CT-1）。

use serde_json::Value;

use super::Push;
use crate::core::kinds::Level;

/// 会话创建（`session.created`）时的权限：只读开着的算只读。别的几格（人格、预设、模型）是开局的，不当成中途换了。
pub(super) fn began(body: &Value, out: &mut Vec<Push>) {
    if let Some(level) = actual(&body["permission"]) {
        out.push(Push::Began(level));
    }
}

/// 一份权限（`{level, read_only}`）实际是哪一级：只读开着的是只读；没有 `level` 的是 `None`。
fn actual(permission: &Value) -> Option<Level> {
    let level = permission["level"].as_str().map(Level::parse)?;
    Some(if permission["read_only"] == true {
        Level::ReadOnly
    } else {
        level
    })
}

/// 读成几样：没带权限的不改级别；认不出的级别照核心按只读画（AUD-4）。换级别的带着换以前的（核心 PL-1 的 `from`，以前的
/// 日志没有）。
pub(super) fn read(body: &Value, out: &mut Vec<Push>) {
    if let Some(level) = body["permission"]["level"].as_str().map(Level::parse) {
        let read_only = body["permission"]["read_only"] == true;
        let from = actual(&body["from"]);
        out.push(Push::Policy {
            level,
            read_only,
            from,
        });
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
    // 压缩触发线（核心 CT-1）：`{"percent"}`，`null` 是不再钉着、跟着配置。
    if let Some(line) = body.get("compaction_line") {
        out.push(Push::CompactionLineSet(line["percent"].as_u64()));
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
