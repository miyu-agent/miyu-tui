use super::{ExportBox, Preview, Use, free_name, preview};
use crate::core::Refusal;

fn open(dir: &std::path::Path) -> ExportBox {
    ExportBox {
        session: "0192f3a0-aaaa-bbbb-cccc-0000deadbeef".into(),
        dir: dir.to_path_buf(),
        previews: std::array::from_fn(|_| Preview::Waiting),
        chosen: None,
    }
}

#[test]
fn files_are_named_after_the_session_and_use_and_never_overwrite() {
    let dir = std::env::temp_dir().join(format!("miyu-export-names-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let open = open(&dir);
    assert_eq!(open.file_name(Use::Agent), "miyu-deadbeef-agent.md");
    assert_eq!(open.file_name(Use::Bundle), "miyu-deadbeef.zip");
    std::fs::write(dir.join("miyu-deadbeef-agent.md"), "").unwrap();
    assert_eq!(
        open.file_name(Use::Agent),
        "miyu-deadbeef-agent-2.md",
        "已经有了的接 -2"
    );
    assert_eq!(free_name(&dir, "x", "md"), dir.join("x.md"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_estimate_keeps_the_text_unless_it_is_too_big_to_copy() {
    let ok = preview(&Ok(
        serde_json::json!({"text": "# Hi", "bytes": 4, "tokens": 2}),
    ));
    assert_eq!(
        ok,
        Preview::Ready {
            bytes: 4,
            tokens: 2,
            text: Some("# Hi".into())
        }
    );
    let big = preview(&Err(Refusal {
        reason: Some("export_too_big".into()),
        message: "too big".into(),
        data: serde_json::Value::Null,
    }));
    assert!(
        matches!(big, Preview::Ready { text: None, .. }),
        "只能存文件"
    );
    let gone = preview(&Err(Refusal {
        reason: Some("session_not_found".into()),
        message: "会话不存在".into(),
        data: serde_json::Value::Null,
    }));
    assert_eq!(gone, Preview::Failed("会话不存在".into()));
}
