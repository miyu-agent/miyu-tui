//! 点开的输出、差异里的制表符（review C1，2026-10-11）：画面库丢掉制表符，原来缩进全没了。照制表位展开成空格。

use std::time::Instant;

use serde_json::json;

use super::{command, rows, segment, step, text, thought};
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

/// 命令的预览（发布前审查 T2-2，2026-10-30）：屏上要和真正要跑的一样，制表符照 8 列展开，看不见的字写成记号。带说明的
/// 命令在说明下面画预览，不带的照命令写；两条路都照它。
#[test]
fn a_command_preview_shows_what_will_really_run() {
    let f = Fixture::new();
    let command = "rm\t-rf\t~/build \u{202e}x";
    for parsed in [
        json!({"command": command, "description": "清理"}),
        json!({"command": command}),
    ] {
        let shell = tool("shell", parsed.clone(), "");
        let shown = text(&rows(0, &segment(vec![shell], Some(true)), &f.ctx()));
        assert!(
            shown
                .iter()
                .any(|l| l.contains("rm      -rf     ~/build <U+202E>x")),
            "{parsed} {shown:?}"
        );
    }
}

/// 命令的预览照真正要跑的画（发布前审查 T2-2）：制表符照 8 列展开，看不见的字写成记号。
#[test]
fn a_command_preview_keeps_its_hidden_characters_visible() {
    let f = Fixture::new();
    let t0 = Instant::now();
    let mut two = command(t0, 1, ToolStatus::Ok);
    if let StepKind::Tool { parsed, .. } = &mut two.kind {
        *parsed = json!({"command": "rm\t-rf\t~/build \u{202e}x", "description": "清理"});
    }
    let seg = segment(vec![thought(t0, 0), two], Some(true));
    let lines = text(&rows(0, &seg, &f.ctx()));
    assert_eq!(
        lines.last().map(String::as_str),
        Some("  │ rm      -rf     ~/build <U+202E>x"),
        "{lines:?}"
    );
}
