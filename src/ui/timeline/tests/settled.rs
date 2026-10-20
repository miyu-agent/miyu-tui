//! 结束了、可里面的命令还在跑的一段（「时间线」第 21 条，2026-10-20 项目主人验收 shellhook 时报：后台命令来了，前台的时间线
//! 收起了，可里面的命令还在跑）：照进行中的样子画，不画收起那一行，跑完了再收。

use std::time::Instant;

use super::{pending, rows, segment, spinning, text, thought};
use crate::transcript::ToolState;
use crate::ui::test_support::Fixture;

#[test]
fn a_finished_segment_with_a_running_command_stays_open_without_a_head() {
    let f = Fixture::new();
    let t0 = Instant::now();
    let seg = segment(vec![thought(t0, 0), pending(ToolState::Running)], None);
    assert!(seg.finished);
    let got = rows(0, &seg, &f.ctx());
    let lines = text(&got);
    // 收起那一行是「等一会 · 1 thought · 1s」：不画它，打头的就是思考那一步。
    assert!(lines[0].contains("已思考"), "不画收起那一行：{lines:?}");
    assert_eq!(spinning(&got, &f.config.timeline.spinner), 1, "{lines:?}");
}
