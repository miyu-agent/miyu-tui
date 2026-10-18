//! 内置语义模型那一步（2026-10-18 项目主人定）：只在装了语义模型的包时出现，没装的往前、往回都跨过去。

use serde_json::json;

use super::{SemanticStep, installed};
use crate::oobe::asker::Waiting;
use crate::oobe::{Move, Step};

fn schema(local: serde_json::Value) -> serde_json::Value {
    json!({"items": [
        {"key": "permission.level", "options": [{"value": "workspace"}]},
        {"key": "models.embedding", "options": [local, {"value": "off"}]}
    ]})
}

#[test]
fn the_package_counts_as_installed_unless_the_local_option_is_unavailable() {
    assert!(installed(&schema(json!({"value": "local"}))));
    assert!(installed(&schema(
        json!({"value": "local", "available": true})
    )));
    assert!(!installed(&schema(
        json!({"value": "local", "available": false})
    )));
    assert!(!installed(
        &json!({"items": [{"key": "models.embedding", "options": [{"value": "off"}]}]})
    ));
    assert!(
        !installed(&json!({"items": []})),
        "核心旧、没有这一项的当没装"
    );
}

#[test]
fn without_the_package_the_step_is_skipped_both_ways() {
    let mut step = SemanticStep::default();
    assert_eq!(
        step.skip(Step::Semantic, false),
        Step::Semantic,
        "还不知道的照常进"
    );
    let got = step.answer(
        &Waiting::SemanticSchema,
        Ok(schema(json!({"value": "local", "available": false}))),
        false,
    );
    assert_eq!(got, Move::Stay);
    assert_eq!(step.skip(Step::Semantic, false), Step::Done, "往前跨到好了");
    assert_eq!(
        step.skip(Step::Semantic, true),
        Step::Preset,
        "往回跨到预设"
    );
    assert_eq!(step.skip(Step::Preset, false), Step::Preset, "别的步不管");
    let mut step = SemanticStep::default();
    step.answer(
        &Waiting::SemanticSchema,
        Ok(schema(json!({"value": "local"}))),
        false,
    );
    assert_eq!(
        step.skip(Step::Semantic, false),
        Step::Semantic,
        "装了的照常进"
    );
}

#[test]
fn standing_on_the_step_when_it_turns_out_missing_moves_on() {
    let mut step = SemanticStep::default();
    let got = step.answer(
        &Waiting::SemanticSchema,
        Ok(schema(json!({"value": "local", "available": false}))),
        true,
    );
    assert_eq!(got, Move::Next);
}

#[test]
fn the_current_setting_is_where_the_pick_starts() {
    let mut step = SemanticStep::default();
    assert_eq!(step.pick.selected, 0, "没写的停在「是」");
    step.answer(
        &Waiting::SemanticRead,
        Ok(json!({"items": {"models.embedding": {"value": "off"}}})),
        true,
    );
    assert_eq!(step.pick.selected, 1, "写了 off 的停在「否」");
}
