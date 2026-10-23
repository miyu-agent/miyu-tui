//! 条目照推送改：照核心 `tests/view_stream.rs` 的 `apply` 那张表。推送照线上的样子写（方法名 + 参数）。

use serde_json::{Value, json};

use super::{Applied, Entries, change_of};
use miyu_client::view::{Body, Entry, EntryId};

const AT: &str = "2026-10-23T08:00:00.000Z";

fn entry(value: Value) -> Entry {
    serde_json::from_value(value).expect("条目读得回来")
}

fn user(id: &str, text: &str) -> Value {
    json!({"id": id, "kind": "user", "text": text, "at": AT})
}

fn reply(id: &str, text: &str) -> Value {
    json!({"id": id, "kind": "reply", "text": text, "open": true, "persona": null, "at": AT})
}

fn push(entries: &mut Entries, method: &str, params: Value) -> Applied {
    let change = change_of(method, &params).unwrap_or_else(|| panic!("读得懂 {method}：{params}"));
    entries.apply(change)
}

fn ids(entries: &Entries) -> Vec<String> {
    entries
        .iter()
        .map(|e| {
            serde_json::to_value(&e.id)
                .unwrap_or_default()
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect()
}

fn id(text: &str) -> EntryId {
    serde_json::from_value(json!(text)).expect("编号")
}

#[test]
fn pushes_add_append_update_hide_and_remove_like_the_cores_table() {
    let mut entries = Entries::page(vec![entry(user("m1", "在吗"))]);
    // 加在 m1 后面；after 不在手里的排最后；null 是最前。
    push(
        &mut entries,
        "view.add",
        json!({"session": "s", "entry": reply("b2.0", "在"), "after": "m1"}),
    );
    push(
        &mut entries,
        "view.add",
        json!({"session": "s", "entry": user("m9", "后来的"), "after": "zz"}),
    );
    push(
        &mut entries,
        "view.add",
        json!({"session": "s", "entry": user("m0", "最前"), "after": null}),
    );
    assert_eq!(ids(&entries), ["m0", "m1", "b2.0", "m9"]);
    // 接字：正文接在 text 后面。
    let got = push(
        &mut entries,
        "view.append",
        json!({"session": "s", "id": "b2.0", "text": "的。"}),
    );
    assert_eq!(got, Applied::Changed(vec![id("b2.0")]));
    let Some(Body::Reply(r)) = entries.get(&id("b2.0")).map(|e| &e.body) else {
        panic!("是正文");
    };
    assert_eq!(r.text, "在的。");
    // 换：没带 after 的原地换；带 after 的挪过去，after 是 null 的挪到最前。
    let mut done = reply("b2.0", "在的。");
    done["open"] = json!(false);
    push(
        &mut entries,
        "view.update",
        json!({"session": "s", "entry": done}),
    );
    assert_eq!(ids(&entries), ["m0", "m1", "b2.0", "m9"]);
    push(
        &mut entries,
        "view.update",
        json!({"session": "s", "entry": user("m9", "后来的"), "after": "m0"}),
    );
    assert_eq!(ids(&entries), ["m0", "m9", "m1", "b2.0"]);
    push(
        &mut entries,
        "view.update",
        json!({"session": "s", "entry": user("m1", "在吗"), "after": null}),
    );
    assert_eq!(
        ids(&entries),
        ["m1", "m0", "m9", "b2.0"],
        "after 是 null：挪到最前"
    );
    // 藏起、显示回来。
    push(
        &mut entries,
        "view.hidden",
        json!({"session": "s", "ids": ["m9", "b2.0", "nope"], "hidden": true}),
    );
    assert!(entries.get(&id("m9")).is_some_and(|e| e.hidden));
    let got = push(
        &mut entries,
        "view.hidden",
        json!({"session": "s", "ids": ["m9"], "hidden": false}),
    );
    assert_eq!(got, Applied::Changed(vec![id("m9")]));
    assert!(entries.get(&id("m9")).is_some_and(|e| !e.hidden));
    // 拿掉。
    push(
        &mut entries,
        "view.remove",
        json!({"session": "s", "id": "m0"}),
    );
    assert_eq!(ids(&entries), ["m1", "m9", "b2.0"]);
}

#[test]
fn tool_args_and_thoughts_grow_and_unknown_targets_ask_for_a_resync() {
    let tool = json!({"id": "b3.1", "kind": "tool", "group": "g3.0", "name": "shell",
        "title": {"name": "运行命令"}, "state": "preparing", "args": "{\"cmd\":", "at": AT});
    let thought = json!({"id": "b3.0", "kind": "thought", "group": "g3.0", "text": "想", "open": true, "at": AT});
    let mut entries = Entries::page(vec![entry(thought), entry(tool)]);
    push(
        &mut entries,
        "view.append",
        json!({"session": "s", "id": "b3.1", "text": "\"ls\"}"}),
    );
    push(
        &mut entries,
        "view.append",
        json!({"session": "s", "id": "b3.0", "text": "一想"}),
    );
    let Some(Body::Tool(t)) = entries.get(&id("b3.1")).map(|e| &e.body) else {
        panic!("是工具");
    };
    assert_eq!(t.args, "{\"cmd\":\"ls\"}");
    let Some(Body::Thought(t)) = entries.get(&id("b3.0")).map(|e| &e.body) else {
        panic!("是思考");
    };
    assert_eq!(t.text, "想一想");
    // 手里没有的：掉了队。
    assert_eq!(
        push(
            &mut entries,
            "view.append",
            json!({"session": "s", "id": "b9.0", "text": "x"})
        ),
        Applied::Unknown
    );
    assert_eq!(
        push(
            &mut entries,
            "view.update",
            json!({"session": "s", "entry": user("m7", "x")})
        ),
        Applied::Unknown
    );
    // 别的推送不是视图流的。
    assert!(change_of("view.status", &json!({"session": "s", "status": {}})).is_none());
    assert!(change_of("session.event", &json!({})).is_none());
    assert_eq!(entries.len(), 2);
}
