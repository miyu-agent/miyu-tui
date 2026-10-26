//! 思考那一行接着写估的 token 数（2026-10-24 项目主人定：只写数字，`12.3k`、`1M`、`1B`；2026-10-26 起想完的也写）。

use std::time::Instant;

use super::{command, rows, segment, step};
use crate::core::ToolStatus;
use crate::transcript::{Step, StepKind};
use crate::ui::test_support::Fixture;

#[test]
fn a_running_thought_shows_its_estimated_tokens_and_a_finished_one_does_not() {
    let f = Fixture::new();
    // 一万二千三百个短英文词，一个词算一个：约 12.3k。
    let text = "abcd ".repeat(12_300);
    let mut running = segment(Vec::new(), None);
    running.finished = false;
    running
        .steps
        .push(Step::new(StepKind::Thought { text: text.clone() }));
    let live = super::text(&rows(0, &running, &f.ctx()));
    assert!(
        live[0].contains("思考中") && live[0].trim_end().ends_with("· 12.3k"),
        "{}",
        live[0]
    );
    // 还没有字的不写 0。
    let mut empty = segment(Vec::new(), None);
    empty.finished = false;
    empty
        .steps
        .push(Step::new(StepKind::Thought { text: " \n".into() }));
    let first = super::text(&rows(0, &empty, &f.ctx()))[0].clone();
    assert!(!first.trim_end().ends_with('0'), "{first}");
    // 想完的也写（2026-10-26 项目主人：原来只在思考中写）。
    let t0 = Instant::now();
    let done = step(StepKind::Thought { text }, t0, 0, 2);
    let seg = segment(vec![done, command(t0, 2, ToolStatus::Ok)], Some(true));
    let lines = super::text(&rows(0, &seg, &f.ctx()));
    let title = lines.iter().find(|l| l.contains("已思考")).unwrap();
    assert!(title.trim_end().ends_with("· 12.3k"), "{title}");
}
