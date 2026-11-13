//! 这个会话改过的文件（`changes.rs`）：只算编辑、写入成了的，按文件归在一起、先改的在前，加减行数照每一步合计；撤销掉的、
//! 没成的、命令改的不算。

use serde_json::{Value, json};

use super::of;
use crate::config::Config;
use crate::core::ToolStatus;
use crate::transcript::{Entry, Kind, Segment, Step, StepKind, ToolState, Transcript};

fn tool(name: &str, args: Value, state: ToolState, call: &str) -> Step {
    let mut step = Step::new(StepKind::Tool {
        name: name.into(),
        args: args.to_string(),
        parsed: args,
        state,
        output: String::new(),
        said: None,
    });
    step.call_id = Some(call.into());
    step
}

fn ok() -> ToolState {
    ToolState::Done(ToolStatus::Ok)
}

fn segment(id: u64, steps: Vec<Step>, hidden: bool) -> Entry {
    let mut e = Entry::new(id, Kind::Steps, String::new());
    let mut s = Segment::new();
    s.steps = steps;
    e.segment = Some(s);
    e.hidden = hidden;
    e
}

#[test]
fn edits_and_writes_group_by_file_in_first_changed_order() {
    let edit = |old: &str, new: &str| json!({"file_path": "/w/a.rs", "edits": [{"old_string": old, "new_string": new}]});
    let mut t = Transcript::default();
    t.entries.push(segment(
        1,
        vec![
            tool("edit", edit("a\n", "b\nc\n"), ok(), "c1"),
            tool(
                "write",
                json!({"file_path": "/w/new.md", "content": "x\ny\nz\n"}),
                ok(),
                "c2",
            ),
            tool(
                "shell",
                json!({"command": "sed -i s/a/b/ /w/a.rs"}),
                ok(),
                "c3",
            ),
        ],
        false,
    ));
    t.entries.push(segment(
        2,
        vec![
            tool("edit", edit("b\n", "d\n"), ok(), "c4"),
            // 没成的、还在跑的不算
            tool("edit", json!({"file_path": "/w/bad.rs", "edits": [{"old_string": "q", "new_string": "r"}]}), ToolState::Done(ToolStatus::Error), "c5"),
            tool("write", json!({"file_path": "/w/run.md", "content": "1\n"}), ToolState::Running, "c6"),
        ],
        false,
    ));
    // 撤销掉的那一轮不算
    t.entries.push(segment(
        3,
        vec![tool(
            "write",
            json!({"file_path": "/w/undone.md", "content": "1\n"}),
            ok(),
            "c7",
        )],
        true,
    ));
    let config = Config::builtin().unwrap();
    let files = of(&t, &config.timeline);
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        ["/w/a.rs", "/w/new.md"],
        "先改的在前、同一个文件只一行"
    );
    let a = &files[0];
    assert_eq!(a.changes.len(), 2, "改了两次");
    assert_eq!((a.added, a.removed), (3, 2), "两次的加减合起来");
    assert_eq!(a.changes[1].call.as_deref(), Some("c4"));
    assert_eq!((files[1].added, files[1].removed), (3, 0));
}
