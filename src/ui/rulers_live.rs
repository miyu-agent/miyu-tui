//! 量尺（`cargo test ruler_live -- --ignored --nocapture`）：进行中的那一段步越多，排一帧要多久（review D1，2026-10-11）。
//! 只打数字，不断言耗时。

use std::time::{Duration, Instant};

use serde_json::json;

use crate::core::ToolStatus;
use crate::transcript::{Segment, Step, StepKind, ToolState};
use crate::ui::test_support::Fixture;
use crate::ui::timeline::rows;

/// 一步编辑：改一个两百行的文件里的一处（点开、预览要算差异）。
fn edit(i: usize) -> Step {
    let before: String = (0..200).map(|n| format!("let line_{n} = {n};\n")).collect();
    let after = before.replace("let line_100 = 100;", &format!("let line_100 = {i};"));
    let mut s = Step::new(StepKind::Tool {
        name: "edit".into(),
        args: String::new(),
        parsed: json!({"path": format!("src/f{i}.rs"), "edits": [{"old_string": before, "new_string": after}]}),
        state: ToolState::Done(ToolStatus::Ok),
        output: String::new(),
        said: None,
    });
    s.took = Some(Duration::from_secs(1));
    s
}

/// 一步思考，几百字。
fn thought() -> Step {
    let mut s = Step::new(StepKind::Thought {
        text: "先看看这个文件，再决定怎么改。\n".repeat(20),
    });
    s.took = Some(Duration::from_secs(2));
    s
}

/// 进行中的一段：思考、编辑轮着来，一共 `n` 步。
fn live(n: usize) -> Segment {
    let mut s = Segment::new();
    s.steps = (0..n)
        .map(|i| if i % 2 == 0 { thought() } else { edit(i) })
        .collect();
    s.finished = false;
    s
}

#[test]
#[ignore]
fn ruler_live_segment() {
    let f = Fixture::new();
    for n in [10usize, 50, 200] {
        let seg = live(n);
        let ctx = f.ctx();
        let frames = 20;
        let start = Instant::now();
        let mut kept = 0;
        for _ in 0..frames {
            kept = rows(0, &seg, &ctx).len();
        }
        let each = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
        println!("进行中的一段 {n} 步：一帧 {each:.2}ms，留下 {kept} 行");
    }
}
