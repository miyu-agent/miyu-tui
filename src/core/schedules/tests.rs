//! 唤醒表里的循环（「循环」第 1 条）：只留跑在已有会话上的循环，一项一项换、删。

use serde_json::json;

use super::{Change, Every, Loop, apply, changed, lost, table};

fn item(id: &str) -> serde_json::Value {
    json!({"id": id, "when": {"every": 600}, "prompt": "看看 CI", "target": {"session": "s-1"},
        "by": {"kind": "person"}, "recurring": true, "paused": false, "loop": "fixed",
        "created": "2026-10-30T00:00:00.000Z", "next": "2026-10-30T04:07:00.000Z"})
}

#[test]
fn only_loops_on_a_session_are_kept() {
    let got = table(&json!({"schedules": [
        item("t1"),
        {"id": "t2", "when": {"cron": "0 8 * * *"}, "prompt": "早报", "target": {"new": {}}, "paused": false},
        {"id": "t3", "when": {}, "prompt": "天气", "target": {"session": "s-2"}, "paused": true, "loop": "dynamic"},
        {"id": "t4", "when": {"cron": "*/5 * * * *"}, "prompt": "x", "target": {"session": "s-1"}, "loop": "fixed"}
    ]}));
    assert_eq!(
        got[0],
        Loop {
            id: "t1".into(),
            session: "s-1".into(),
            every: Every::Seconds(600),
            next: Some("2026-10-30T04:07:00Z".parse().unwrap()),
            paused: false,
            prompt: "看看 CI".into(),
        }
    );
    assert_eq!(got.len(), 3, "定时任务不算");
    assert_eq!(
        (got[1].every.clone(), got[1].paused, got[1].next),
        (Every::Dynamic, true, None)
    );
    assert_eq!(got[2].every, Every::Cron("*/5 * * * *".into()));
}

#[test]
fn changes_replace_add_and_remove_one_by_one() {
    let mut loops = table(&json!({"schedules": [item("t1")]}));
    let push = |params| json!({"method": "schedules.changed", "params": params});
    let mut paused = item("t1");
    paused["paused"] = json!(true);
    apply(
        &mut loops,
        changed(&push(json!({"id": "t1", "entry": paused}))).unwrap(),
    );
    assert!(loops[0].paused, "换上");
    apply(
        &mut loops,
        changed(&push(json!({"id": "t2", "entry": item("t2")}))).unwrap(),
    );
    assert_eq!(loops.len(), 2, "加上");
    apply(
        &mut loops,
        changed(&push(json!({"id": "t1", "removed": true}))).unwrap(),
    );
    assert_eq!(
        loops.iter().map(|l| l.id.as_str()).collect::<Vec<_>>(),
        ["t2"],
        "删掉"
    );
    let mut stopped = item("t2");
    stopped.as_object_mut().unwrap().remove("loop");
    apply(
        &mut loops,
        changed(&push(json!({"id": "t2", "entry": stopped}))).unwrap(),
    );
    assert!(loops.is_empty(), "不是循环了的照删掉办");
    apply(&mut loops, Change::All(vec![]));
    assert!(changed(&json!({"method": "notice", "params": {}})).is_none());
    assert!(lost(
        &json!({"method": "resync", "params": {"stream": "schedules"}})
    ));
}
