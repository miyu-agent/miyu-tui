//! 问顾问那一步（核心 ADV-2，设计 69，ADV-3）：点开看问答——问的话，空一行，顾问的回答照 Markdown 画。

use std::time::Instant;

use serde_json::json;

use super::{rows, segment, step, text, thought};
use crate::core::ToolStatus;
use crate::transcript::{StepKind, ToolState};
use crate::ui::test_support::Fixture;

#[test]
fn an_advisor_step_opens_on_the_question_and_the_answer() {
    let f = Fixture::new();
    let t0 = Instant::now();
    let mut advice = step(
        StepKind::Tool {
            name: "advisor".into(),
            args: String::new(),
            parsed: json!({"question": "缓存老是断，先查哪？"}),
            state: ToolState::Done(ToolStatus::Ok),
            output: "先看 **工具面**有没有在会话中途变。".into(),
            said: None,
        },
        t0,
        0,
        1,
    );
    advice.open = Some(true);
    let lines = text(&rows(
        0,
        &segment(vec![thought(t0, 0), advice], Some(true)),
        &f.ctx(),
    ));
    let asked = lines
        .iter()
        .position(|l| l.contains("缓存老是断，先查哪？"));
    let answered = lines
        .iter()
        .position(|l| l.contains("先看 工具面有没有在会话中途变。"));
    assert!(
        asked.zip(answered).is_some_and(|(a, b)| b > a + 1),
        "问的话在上、空一行、回答照 Markdown 画（不留 **）：{lines:?}"
    );
}
