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

#[test]
fn what_the_memory_module_kept_reads_for_the_receipt_and_others_do_not() {
    let line = |kind: &str, by: Value, body: Value| {
        json!({"method": "memory.event", "params": {"session": "s1",
            "event": {"seq": 9, "kind": kind, "by": by, "body": body}}})
    };
    let module = json!({"kind": "module", "id": "memory"});
    assert_eq!(
        change(line(
            "ext.memory.saved",
            module.clone(),
            json!({"text": "橘猫", "replaces": "m2"})
        )),
        Some(Change::Landed(Landed::Revised("橘猫".into())))
    );
    assert_eq!(
        change(line(
            "ext.memory.retired",
            module.clone(),
            json!({"id": "m1", "kind": "outdated", "why": "x"})
        )),
        Some(Change::Landed(Landed::Retired(
            "m1".into(),
            Retire::Outdated
        )))
    );
    assert_eq!(
        change(line(
            "ext.memory.summary",
            module.clone(),
            json!({"text": "用户养猫。"})
        )),
        Some(Change::Landed(Landed::Summary("用户养猫。".into())))
    );
    // 抽出来的新记的（没有 `replaces`）、她在会话里记的、人删的都不算这次整理。
    let fresh = line("ext.memory.saved", module, json!({"text": "新记的"}));
    assert!(matches!(pushed(&fresh), Some(None)));
    let by_tool = line(
        "ext.memory.saved",
        json!({"kind": "tool"}),
        json!({"text": "x", "replaces": "m2"}),
    );
    assert!(matches!(pushed(&by_tool), Some(None)));
    let by_person = line(
        "ext.memory.retired",
        json!({"kind": "person"}),
        json!({"id": "m1"}),
    );
    assert!(matches!(pushed(&by_person), Some(None)));
}
