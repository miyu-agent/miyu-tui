//! 一轮进行中来的回报是时间线里的一步（发版前 check 21，2026-11-09 项目主人从三个样板里挑了 A）：收起那一行照类多数一格，
//! 展开在它发生的位置画一行（记号照原来的颜色）；只有回报的一段不收成一行。

use std::time::Instant;

use super::{command, rows, segment, step, text, thought};
use crate::core::ToolStatus;
use crate::theme;
use crate::transcript::{JobMark, JobNote, ReportClass, StepKind};
use crate::ui::test_support::Fixture;

fn report(t0: Instant, start: u64, class: ReportClass) -> crate::transcript::Step {
    let kind = StepKind::Report {
        note: JobNote {
            mark: JobMark::Done,
            detail: String::new(),
            source: None,
        },
        text: "后台命令完成 · sleep 20 · 20s".into(),
        class,
    };
    step(kind, t0, start, 0)
}

#[test]
fn the_folded_line_counts_the_reports_and_the_open_one_shows_where_it_came() {
    let f = Fixture::new();
    let t0 = Instant::now();
    let steps = vec![
        command(t0, 0, ToolStatus::Ok),
        report(t0, 1, ReportClass::Command),
        thought(t0, 1),
        command(t0, 2, ToolStatus::Ok),
    ];
    let mut ctx = f.ctx();
    ctx.width = 100;
    let folded = text(&rows(0, &segment(steps.clone(), None), &ctx));
    assert_eq!(
        folded,
        ["  Ran 2 commands · 1 thought · 1 background command finished · 3s"]
    );
    let open = rows(0, &segment(steps, Some(true)), &ctx);
    let lines = text(&open);
    let at = lines
        .iter()
        .position(|l| l.contains("● 后台命令完成"))
        .expect("展开看得到回报");
    let thought_at = lines.iter().position(|l| l.contains("已思考")).unwrap();
    assert!(at < thought_at, "画在它发生的位置：{lines:?}");
    assert!(
        open[at]
            .line
            .spans
            .iter()
            .any(|s| s.content == "● " && s.style == theme::good()),
        "记号照原来那一行的颜色"
    );
}

#[test]
fn a_segment_of_reports_only_is_not_folded_into_one_line() {
    let f = Fixture::new();
    let t0 = Instant::now();
    let seg = segment(
        vec![
            report(t0, 0, ReportClass::Command),
            report(t0, 1, ReportClass::Task),
        ],
        None,
    );
    let lines = text(&rows(0, &seg, &f.ctx()));
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.contains("● 后台命令完成"))
            .count(),
        2,
        "照原来单独的那几行画：{lines:?}"
    );
}
