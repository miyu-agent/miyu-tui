//! 等人确认、答题的那几段不算进用时（发版前 check，2026-11-12 项目主人；口径同核心施工 9-8 十四补）。

use std::time::Duration;

use jiff::{SignedDuration, Timestamp};
use serde_json::json;

use super::apply;
use crate::core::{Asking, EndReason, Push};
use crate::transcript::{Kind, Transcript};

#[test]
fn the_done_line_leaves_out_the_time_spent_waiting_on_a_person() {
    // 开轮，2 秒后要确认，等了一分钟才点，再过 3 秒结束：用时 5 秒，不是 65 秒。
    let t0 = Timestamp::now() - SignedDuration::from_secs(3600);
    let at = |secs: i64| Push::Clock(Some(t0 + SignedDuration::from_secs(secs)));
    let mut t = Transcript::default();
    apply(
        &mut t,
        vec![
            Push::Model {
                endpoint: "deepseek".into(),
                model: "deepseek-flash".into(),
            },
            at(0),
            Push::TurnStarted(1, None),
            at(2),
            Push::Asking(Asking::Approval(json!({"call_id": "c1"}))),
            at(62),
            Push::Asking(Asking::Decided(json!({"call_id": "c1"}))),
            at(65),
            Push::TurnEnded(EndReason::Completed),
            Push::Clock(None),
        ],
    );
    let done = t.entries.last().unwrap();
    assert_eq!(done.kind, Kind::Done);
    let took = done.text.split(" · ").nth(2).unwrap();
    assert_eq!(
        took,
        crate::meter::seconds(Duration::from_secs(5)),
        "{}",
        done.text
    );
}

#[test]
fn the_running_time_stands_still_while_a_person_is_asked() {
    let mut t = Transcript::default();
    apply(&mut t, vec![Push::TurnStarted(1, None)]);
    let start = t.running.unwrap();
    apply(
        &mut t,
        vec![Push::Asking(Asking::Asked(json!({"call_id": "q1"})))],
    );
    assert!(t.waiting_on_person());
    let later = start + Duration::from_secs(90);
    assert!(
        t.ran(later).unwrap() < Duration::from_secs(1),
        "等人时用时不走"
    );
    // 提问那一次调用有了结果（答了），接着走。
    apply(
        &mut t,
        vec![Push::ToolResult {
            call_id: "q1".into(),
            status: crate::core::ToolStatus::Ok,
            text: String::new(),
            said: None,
            display: Default::default(),
            sent: 0,
            detail: None,
        }],
    );
    assert!(!t.waiting_on_person());
}
