use serde_json::json;

use super::*;

fn change(message: Value) -> Option<Change> {
    match pushed(&message)? {
        Some(Update::Dream { session, change }) => {
            assert_eq!(session, "s1");
            Some(change)
        }
        _ => None,
    }
}

#[test]
fn the_three_log_lines_read_with_their_batch() {
    let line = |kind: &str, body: Value| {
        json!({"method": "memory.event", "params": {"session": "s1",
            "event": {"seq": 9, "kind": kind, "body": body}}})
    };
    assert_eq!(
        change(line(
            "ext.memory.merging",
            json!({"upto": 12, "fresh": 12, "left": 48, "manual": true, "given": ["m3", "m7"]})
        )),
        Some(Change::Merging {
            upto: 12,
            fresh: 12,
            left: 48,
            given: vec!["m3".into(), "m7".into()]
        })
    );
    assert_eq!(
        change(line("ext.memory.merged", json!({"upto": 12, "given": 14}))),
        Some(Change::Merged { upto: 12 })
    );
    assert_eq!(
        change(line(
            "ext.memory.merge_failed",
            json!({"upto": 12, "why": "x"})
        )),
        Some(Change::Failed { upto: 12 })
    );
    // 别的日志行是记忆流的、和整理无关：截下来，不往下走。
    let saved = line("ext.memory.saved", json!({"text": "x"}));
    assert!(matches!(pushed(&saved), Some(None)));
}

#[test]
fn what_she_writes_reads_the_three_kinds() {
    let push = |params: Value| json!({"method": "memory.dreaming", "params": params});
    assert_eq!(
        change(push(
            json!({"upto": 2, "kind": "revised", "id": "m2", "text": "橘猫", "session": "s1"})
        )),
        Some(Change::Revised {
            text: "橘猫".into()
        })
    );
    assert_eq!(
        change(push(
            json!({"upto": 2, "kind": "retired", "id": "m1", "retire": "repeat", "kept": "m2",
            "why": "重复", "session": "s1"})
        )),
        Some(Change::Retired {
            id: "m1".into(),
            why: Retire::Repeat
        })
    );
    assert_eq!(
        change(push(
            json!({"upto": 2, "kind": "summary", "chars": 40, "done": false, "session": "s1"})
        )),
        Some(Change::Summary {
            chars: 40,
            done: false
        })
    );
}

#[test]
fn other_methods_are_not_ours_and_the_list_reads_ids_to_texts() {
    assert!(pushed(&json!({"method": "event", "params": {"session": "s1"}})).is_none());
    let Update::Dream {
        change: Change::Texts(texts),
        ..
    } = texts(
        "s1".into(),
        &json!({"memories": [{"id": "m1", "text": "黑咖啡"}, {"id": "m2"}]}),
    )
    else {
        panic!("not texts");
    };
    assert_eq!(texts.len(), 1);
    assert_eq!(texts["m1"], "黑咖啡");
}
