//! 只有一步的一段（蓝图「时间线」第 15 条）：点收起那一行直接铺开这一步，工具照常画它的标题行。

use std::time::Instant;

use super::{command, rows, segment, text, thought};
use crate::core::ToolStatus;
use crate::ui::test_support::Fixture;

#[test]
fn a_one_step_segment_opens_straight_into_its_content() {
    let f = Fixture::new();
    let t0 = Instant::now();
    let seg = segment(vec![thought(t0, 0)], Some(true));
    let rows = rows(0, &seg, &f.ctx());
    assert_eq!(text(&rows), ["  Thought for 1s", "", "    先看看目录", ""]);
    assert!(rows.iter().all(|r| r.shade), "连收起那一行一起铺底色");
    // 工具照常画标题行再接内容（2026-10-18 项目主人：只有收起那一行看不出是哪件、结果是什么）。
    let seg = segment(vec![command(t0, 0, ToolStatus::Ok)], Some(true));
    let rows = super::rows(0, &seg, &f.ctx());
    let lines = text(&rows);
    assert_eq!(lines[0], "  列目录 · 1s");
    // 和收起那一行之间隔一行连接线，同几步的一段（2026-10-23 项目主人：「Used 1 tool」和「抖音下载」之间没有空行）。
    assert_eq!(lines[1].trim(), "│", "{lines:?}");
    assert!(lines[2].contains("shell · 列目录"), "{lines:?}");
    assert!(lines.iter().any(|l| l.contains("a.txt")), "{lines:?}");
    assert!(
        rows.iter()
            .all(|r| r.shade && r.target == Some(crate::ui::rows::Target::Segment(0)))
    );
}
