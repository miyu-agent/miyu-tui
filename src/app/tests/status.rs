//! `/status` 开框时问这个会话每个模型用了多少（2026-10-26 项目主人：中途换过模型的分不出每个模型的用量）。

use serde_json::json;

use super::app;
use crate::core::{Command, Update};

#[test]
fn opening_status_asks_for_usage_by_model_and_lists_what_comes_back() {
    let (mut app, mut sent) = app();
    app.talk.transcript.session = Some("s1".into());
    app.open_status();
    let (tag, params) = std::iter::from_fn(|| sent.try_recv().ok())
        .find_map(|command| match command {
            Command::Ask {
                tag,
                method: "usage.query",
                params,
                ..
            } => Some((tag, params)),
            _ => None,
        })
        .expect("开框时问");
    assert_eq!(
        params,
        json!({"session": "s1", "tree": true, "group": ["model"]})
    );
    let row = |model: &str| json!({"model": model, "requests": 1, "usage": {"uncached": 1000, "output": 10}});
    app.core(Update::Answer {
        tag,
        result: Ok(json!({"rows": [row("dev/a"), row("dev/b")]})),
    });
    let facts = crate::ui::status_box::facts(&app.seen());
    let models: Vec<&str> = facts.models.iter().map(|(m, ..)| m.as_str()).collect();
    assert_eq!(models, ["dev/a", "dev/b"]);
}
