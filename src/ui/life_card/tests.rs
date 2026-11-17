//! 人生卡上写什么（「推演人生」第 7 条）：不写第几轮，暂停着的不写下次继续。

use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::{fields, labels};
use crate::config::Config;
use crate::core::{Loop, LoopEvery, LoopLife};

fn row(paused: bool) -> Loop {
    Loop {
        id: "t9".into(),
        session: "s".into(),
        every: LoopEvery::Life,
        next: Some("2026-11-17T09:00:00Z".parse().unwrap()),
        paused,
        prompt: String::new(),
        life: Some(LoopLife {
            persona: Some("miyu".into()),
            created: Some("2026-11-16T06:00:00Z".parse().unwrap()),
            last: Some("2026-11-17T08:00:00Z".parse().unwrap()),
        }),
    }
}

fn text(row: &Loop) -> Vec<String> {
    let config = Config::builtin().unwrap();
    let now: Timestamp = "2026-11-17T08:30:00Z".parse().unwrap();
    fields(row, Some("美羽"), &config, (now, &TimeZone::UTC))
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn the_card_lists_who_when_and_state_but_no_round() {
    assert_eq!(
        text(&row(false)),
        [
            "人格      美羽",
            "开始      11-16 06:00",
            "下次继续  09:00",
            "上一轮    08:00",
            "状态      进行中",
        ]
    );
    assert_eq!(
        text(&row(true)),
        [
            "人格      美羽",
            "开始      11-16 06:00",
            "上一轮    08:00",
            "状态      已暂停",
        ],
        "暂停着的不写下次"
    );
}

#[test]
fn the_buttons_follow_the_state() {
    let config = Config::builtin().unwrap();
    assert_eq!(labels(&config, false, false), ["暂停", "结束人生"]);
    assert_eq!(labels(&config, true, true), ["继续", "确认结束"]);
}
