//! 读、写、改文件的参数照标准写法规整（核心 ED-3）：别名、顶层直接写一处的编辑，差异、侧栏照规整过的算；别的工具的
//! `path` 不碰，参数原文不动。

use super::*;

/// 调一次 `name`，参数是 `args`，收全。交回这一步。
fn called(name: &str, args: &str) -> crate::transcript::Step {
    let mut t = Transcript::default();
    apply(
        &mut t,
        vec![
            Push::TurnStarted(1, None),
            Push::BlockStart {
                index: 0,
                block: Block::ToolCall(name.into()),
            },
            Push::Delta {
                index: 0,
                text: args.into(),
            },
            Push::BlockEnd(0),
        ],
    );
    t.entries[0].segment.as_ref().unwrap().steps[0].clone()
}

#[test]
fn an_edit_written_with_aliases_is_read_the_standard_way() {
    let raw = r#"{"path":"a.txt","oldText":"x","newText":"y","replaceAll":"true"}"#;
    let step = called("edit", raw);
    assert_eq!(step.arg("file_path"), Some("a.txt"), "对象照标准名取得到");
    let StepKind::Tool { args, parsed, .. } = &step.kind else {
        panic!("是工具");
    };
    assert_eq!(args, raw, "原文不动");
    assert_eq!(parsed["edits"][0]["old_string"], "x");
    assert_eq!(parsed["edits"][0]["new_string"], "y");
    let diff = crate::diff::from_args(parsed).expect("算得出差异");
    assert_eq!((diff.added, diff.removed), (1, 1), "卡片上 +1 -1");
}

#[test]
fn a_write_and_a_read_with_path_are_read_the_standard_way() {
    let write = called("write", r#"{"path":"b.txt","content":"一\n二\n"}"#);
    assert_eq!(write.arg("file_path"), Some("b.txt"));
    let read = called("read", r#"{"filePath":"c.txt"}"#);
    assert_eq!(read.arg("file_path"), Some("c.txt"));
}

#[test]
fn other_tools_keep_their_own_path() {
    // `grep` 的 `path` 是它自己的标准名：不改成 `file_path`。
    let grep = called("grep", r#"{"pattern":"x","path":"src"}"#);
    assert_eq!(
        (grep.arg("path"), grep.arg("file_path")),
        (Some("src"), None)
    );
}
