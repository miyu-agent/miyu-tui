//! 她写正文时来的回报（2026-11-17 项目主人定）：正文后面另起一段只有回报的时间线，一轮就成了「正文 → ◆回报 → ⏵⏵收尾」。一轮
//! 结束时这一段挪到收尾行后面，一条一行，同一轮结束以后来的回报；回报后面还跟着工具的，照旧留在时间线里。

use super::super::{JobMark, Kind, ReportClass, StepKind, Transcript};
use super::apply;
use crate::core::{Block, EndReason, Push};

/// 开一轮、跑一条命令、开始写正文。
fn writing(t: &mut Transcript) {
    apply(
        t,
        vec![
            Push::TurnStarted(1, None),
            Push::BlockStart {
                index: 0,
                block: Block::ToolCall("shell".into()),
            },
            Push::Delta {
                index: 0,
                text: "{\"command\":\"sleep 3\",\"description\":\"等三秒\"}".into(),
            },
            Push::BlockEnd(0),
            Push::Calls(vec!["c1".into()]),
            Push::BlockStart {
                index: 1,
                block: Block::Text,
            },
            Push::Delta {
                index: 1,
                text: "正在写".into(),
            },
        ],
    );
}

fn report(t: &mut Transcript, source: &str) {
    t.job_from(
        JobMark::Done,
        format!("后台命令完成 · {source}"),
        String::new(),
        (Some(source.into()), ReportClass::Command),
    );
}

fn kinds(t: &Transcript) -> Vec<Kind> {
    t.entries.iter().map(|e| e.kind.clone()).collect()
}

fn reports_in_timeline(t: &Transcript) -> usize {
    let segments = t.entries.iter().filter_map(|e| e.segment.as_ref());
    segments
        .flat_map(|s| s.steps.iter())
        .filter(|step| matches!(step.kind, StepKind::Report { .. }))
        .count()
}

#[test]
fn reports_that_came_while_writing_go_after_the_closing_line() {
    let mut t = Transcript::default();
    writing(&mut t);
    report(&mut t, "j1");
    report(&mut t, "j2");
    assert_eq!(
        kinds(&t),
        [Kind::Steps, Kind::Reply, Kind::Steps],
        "写着的时候照旧另起一段"
    );
    apply(
        &mut t,
        vec![
            Push::Delta {
                index: 1,
                text: "，写完了".into(),
            },
            Push::BlockEnd(1),
            Push::TurnEnded(EndReason::Completed),
        ],
    );
    assert_eq!(
        kinds(&t),
        [Kind::Steps, Kind::Reply, Kind::Done, Kind::Job, Kind::Job],
        "正文 → 收尾 → 回报"
    );
    assert_eq!(reports_in_timeline(&t), 0);
    let job = t.entries[3].job.as_ref().expect("回报那一行带着通知");
    assert_eq!(job.source.as_deref(), Some("j1"));
    assert_eq!(t.entries[4].text, "后台命令完成 · j2");
    // 输出读回来照旧照任务编号补进那一行。
    t.fill_job("j2", "done".into());
    assert_eq!(t.entries[4].job.as_ref().unwrap().detail, "done");
}

#[test]
fn reports_followed_by_tools_stay_in_the_timeline() {
    let mut t = Transcript::default();
    writing(&mut t);
    report(&mut t, "j1");
    apply(
        &mut t,
        vec![
            Push::BlockEnd(1),
            Push::BlockStart {
                index: 2,
                block: Block::ToolCall("shell".into()),
            },
            Push::Delta {
                index: 2,
                text: "{\"command\":\"ls\",\"description\":\"看看\"}".into(),
            },
            Push::BlockEnd(2),
            Push::Calls(vec!["c2".into()]),
            Push::TurnEnded(EndReason::Interrupted),
        ],
    );
    assert_eq!(
        kinds(&t),
        [Kind::Steps, Kind::Reply, Kind::Steps, Kind::Done]
    );
    assert_eq!(reports_in_timeline(&t), 1, "回报和后面的工具在同一段里");
}
