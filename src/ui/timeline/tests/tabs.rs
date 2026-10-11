//! 点开的输出、差异里的制表符（review C1，2026-10-11）：画面库丢掉制表符，原来缩进全没了。照制表位展开成空格。

use std::time::Instant;

use serde_json::json;

use super::{rows, segment, step, text};
use crate::core::ToolStatus;
use crate::transcript::{StepKind, ToolState};
use crate::ui::test_support::Fixture;

fn tool(name: &str, parsed: serde_json::Value, output: &str) -> crate::transcript::Step {
    step(
        StepKind::Tool {
            name: name.into(),
            args: String::new(),
            parsed,
            state: ToolState::Done(ToolStatus::Ok),
            output: output.into(),
            said: None,
        },
        Instant::now(),
        0,
        1,
    )
}

#[test]
fn a_command_output_keeps_its_tab_indent() {
    let f = Fixture::new();
    let shell = tool(
        "shell",
        json!({"command": "cat Makefile"}),
        "all:\n\tcc main.c",
    );
    let shown = text(&rows(0, &segment(vec![shell], Some(true)), &f.ctx()));
    assert!(shown.iter().all(|l| !l.contains('\t')), "{shown:?}");
    assert!(
        shown.iter().any(|l| l.ends_with("    cc main.c")),
        "{shown:?}"
    );
}

#[test]
fn a_diff_keeps_its_tab_indent() {
    let f = Fixture::new();
    let edits = json!([{"old_string": "func f() {\n}", "new_string": "func f() {\n\tg()\n}"}]);
    let edit = tool("edit", json!({"path": "a.go", "edits": edits}), "");
    let shown = text(&rows(0, &segment(vec![edit], Some(true)), &f.ctx()));
    assert!(shown.iter().all(|l| !l.contains('\t')), "{shown:?}");
    assert!(shown.iter().any(|l| l.ends_with("+     g()")), "{shown:?}");
}
