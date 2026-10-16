//! `/jump` 列的是哪几句（蓝图「跳到说过的话」第 1 条）。

use std::time::SystemTime;

use super::spoken;
use crate::transcript::{Entry, Kind};

fn entry(id: u64, kind: Kind, text: &str) -> Entry {
    Entry::new(id, kind, text.into())
}

#[test]
fn what_you_said_is_listed_in_order_with_its_time_and_undone_turns_are_left_out() {
    let at: jiff::Timestamp = "2026-10-16T08:00:00Z".parse().unwrap();
    let mut first = entry(1, Kind::User, "第一句");
    first.at = Some(at);
    let mut undone = entry(4, Kind::User, "撤掉的");
    undone.hidden = true;
    let mut foreign = entry(5, Kind::User, "子代理的交代");
    foreign.from = Some("从 j1 收到".into());
    let entries = [
        first,
        entry(2, Kind::Reply, "她的回答"),
        entry(3, Kind::User, "第二句"),
        undone,
        foreign,
    ];
    let got = spoken(&entries);
    let texts: Vec<(&str, u64)> = got
        .iter()
        .map(|(s, id)| (s.draft.text.as_str(), *id))
        .collect();
    assert_eq!(texts, [("第一句", 1), ("第二句", 3), ("子代理的交代", 5)]);
    assert_eq!(got[0].0.at, SystemTime::from(at), "照说的那一刻");
}
