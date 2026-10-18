//! 展开的时间线点空白收起（2026-10-19 项目主人）：没铺底色的行空白收起整段，点开的一步铺了底色的空白收起那一步。

use std::time::Instant;

use serde_json::json;

use super::{rows, segment, step, thought};
use crate::core::ToolStatus;
use crate::transcript::{StepKind, ToolState};
use crate::ui::rows::Target;
use crate::ui::test_support::Fixture;

#[test]
fn an_open_segment_marks_its_blank_to_fold_the_whole_segment() {
    let f = Fixture::new();
    let t0 = Instant::now();
    let mut read = step(
        StepKind::Tool {
            name: "read".into(),
            args: String::new(),
            parsed: json!({"path": "a.rs"}),
            state: ToolState::Done(ToolStatus::Ok),
            output: "fn main() {}".into(),
            said: None,
        },
        t0,
        1,
        1,
    );
    read.open = Some(true);
    let got = rows(
        5,
        &segment(vec![thought(t0, 0), read], Some(true)),
        &f.ctx(),
    );
    assert_eq!(got[0].target, Some(Target::Segment(5)), "收起那一行");
    for row in &got {
        let want = if row.shade {
            Some(Target::Step(5, 1))
        } else {
            Some(Target::Segment(5))
        };
        assert_eq!(row.blank, want, "{:?}", row.plain);
    }
    assert!(got.iter().any(|r| r.shade), "点开的那一步铺了底色");
    // 收着的一段只有收起那一行，不另标。
    let folded = rows(5, &segment(vec![thought(t0, 0)], None), &f.ctx());
    assert!(folded.iter().all(|r| r.blank.is_none()));
}
