use crate::core::{JobEnd, JobReason, JobStart, Push, Update};

use super::super::spool::Record;
use super::{Follow, Watch};

fn start(job: &str, agent: bool) -> JobStart {
    JobStart {
        call_id: format!("call_{job}"),
        job: job.into(),
        agent,
        title: format!("任务 {job}"),
        session: None,
        foreground: false,
    }
}

fn end(job: &str) -> JobEnd {
    JobEnd {
        job: job.into(),
        reason: JobReason::Finished,
        exit_code: Some(0),
        signal: None,
        duration_ms: Some(1000),
        text: String::new(),
    }
}

fn follow(jobs: Vec<JobStart>) -> Follow {
    let mut follow = Follow::new(&Watch {
        session: "s".into(),
        jobs,
        shell: 1,
    });
    follow.update(Update::Paged(false));
    follow
}

#[test]
fn only_the_jobs_handed_over_are_reported() {
    // 几个 shell 共用终端会话：只认派它的那个 shell 交来的任务编号（设计 22 第七节）。
    let mut follow = follow(vec![start("j1", false)]);
    assert!(
        follow
            .update(Update::Push(Push::JobEnded(end("j9"))))
            .is_empty(),
        "别的 shell 派的"
    );
    let got = follow.update(Update::Push(Push::JobEnded(end("j1"))));
    assert!(matches!(got.as_slice(), [Record::Ended { start, .. }] if start.job == "j1"));
    assert!(!follow.finished(), "后台命令跑完也叫醒她：等那一轮");
    let got = follow.update(Update::Push(Push::TurnStarted(5, None)));
    assert_eq!(
        got,
        vec![Record::Turn {
            session: "s".into(),
            turn: 5
        }]
    );
    follow.update(Update::Push(Push::TurnEnded(
        crate::core::EndReason::Completed,
    )));
    assert!(follow.finished());
}

#[test]
fn an_agent_report_hands_over_the_turn_it_wakes() {
    let mut follow = follow(vec![start("j1", true)]);
    follow.update(Update::Push(Push::JobEnded(end("j1"))));
    assert!(!follow.finished(), "还等它叫醒的那一轮");
    // 别的 shell 里你说的话开的那一轮不算。
    follow.update(Update::Push(Push::UserMessage(40)));
    assert!(
        follow
            .update(Update::Push(Push::TurnStarted(41, Some(40))))
            .is_empty()
    );
    let got = follow.update(Update::Push(Push::TurnStarted(43, Some(42))));
    assert_eq!(
        got,
        vec![Record::Turn {
            session: "s".into(),
            turn: 43
        }]
    );
    // 这一轮里又派出去的接着盯。
    follow.update(Update::Push(Push::JobStarted(start("j2", false))));
    follow.update(Update::Push(Push::TurnEnded(
        crate::core::EndReason::Completed,
    )));
    assert!(!follow.finished(), "j2 还在跑");
    follow.update(Update::Push(Push::JobEnded(end("j2"))));
    assert!(!follow.finished(), "j2 跑完了也叫醒她");
    follow.update(Update::Push(Push::TurnStarted(50, None)));
    follow.update(Update::Push(Push::TurnEnded(
        crate::core::EndReason::Completed,
    )));
    assert!(follow.finished());
}

#[test]
fn the_replayed_page_is_not_reported() {
    let mut follow = Follow::new(&Watch {
        session: "s".into(),
        jobs: vec![start("j1", false)],
        shell: 1,
    });
    assert!(
        follow
            .update(Update::Push(Push::JobEnded(end("j1"))))
            .is_empty(),
        "补发的那一页里的"
    );
    assert!(!follow.finished());
}
