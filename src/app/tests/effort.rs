//! 这个会话自己定的思考强度（核心 SM-1，`app/effort.rs`）：底栏照它，`model.list` 回来不拿配置的那一档盖掉；框里标它。

use serde_json::json;

use super::app;
use crate::core::{Current, EffortList, Update};

fn list() -> EffortList {
    EffortList::read(&json!({
        "uses": {"chat": "dev/alpha"},
        "providers": [{"id": "dev", "models": [
            {"model": "alpha", "ref": "dev/alpha", "facts": {
                "reasoning": {"value": ["off", "low", "high"]},
                "effort": {"value": "high", "from": "system", "key": "providers.dev.models.alpha.effort"}}}]}]}))
}

#[test]
fn the_sessions_own_effort_is_not_covered_by_the_configured_one() {
    let (mut app, _sent) = app();
    let current = Current {
        endpoint: Some("dev".into()),
        model: Some("alpha".into()),
        reference: "dev/alpha".into(),
        effort: Some("low".into()),
        effort_own: true,
    };
    app.talk
        .transcript
        .update(Update::CurrentModel(current), &app.config.text);
    app.efforts_listed(list());
    assert_eq!(
        app.talk.transcript.effort(),
        Some("low"),
        "会话自己定的照它"
    );
    // 回到跟随设置：底栏照配置的那一档。
    app.effort_set(None);
    app.efforts_listed(list());
    assert_eq!(app.talk.transcript.effort(), Some("high"));
}
