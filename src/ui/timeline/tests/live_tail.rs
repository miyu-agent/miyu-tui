//! 限制着的进行中那一段只排露出来的那几步（review D1，2026-10-11）：原来每帧把所有步整份排一遍，最后只留十几行，步越多
//! 越卡。排出来的要和整份排完再截一模一样。

use std::time::Instant;

use super::{command, rows, step, text, thought};
use crate::core::ToolStatus;
use crate::transcript::{Segment, StepKind};
use crate::ui::test_support::Fixture;

fn live(n: usize) -> Segment {
    let t0 = Instant::now();
    let mut s = Segment::new();
    s.steps = (0..n)
        .map(|i| match i % 3 {
            0 => thought(t0, i as u64),
            1 => command(
                t0,
                i as u64,
                if i % 7 == 1 {
                    ToolStatus::Error
                } else {
                    ToolStatus::Ok
                },
            ),
            _ => step(
                StepKind::Thought {
                    text: format!("第 {i} 步\n").repeat(i % 5 + 1),
                },
                t0,
                i as u64,
                1,
            ),
        })
        .collect();
    s.finished = false;
    s
}

#[test]
fn only_the_steps_that_show_are_laid_out_and_the_rows_are_the_same() {
    let f = Fixture::new();
    for n in [1usize, 2, 3, 5, 8, 13, 40] {
        let seg = live(n);
        let limited = rows(0, &seg, &f.ctx());
        let mut whole = Fixture::new();
        whole.config.timeline.limit_live = false;
        let all = rows(0, &seg, &whole.ctx());
        let cap = super::super::live_cap(&f.config.timeline);
        let expected = text(&all[all.len().saturating_sub(cap)..]);
        assert_eq!(text(&limited), expected, "{n} 步");
        assert!(limited.iter().all(|r| r.target.is_none()), "{n} 步：点不开");
    }
}
