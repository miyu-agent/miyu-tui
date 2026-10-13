//! 编辑那一步（蓝图 `tui.md`「时间线」第 10 条）：被拒的编辑是出错、不加行；收起那一行带增删的行数。从 `mod.rs` 挪出来
//! （2026-10-11：过了 500 行）。

use std::time::Instant;

use super::super::step;
use super::{edit, rows, segment, text, thought};
use crate::core::ToolStatus;
use crate::theme;
use crate::transcript::{StepKind, ToolState};
use crate::ui::test_support::Fixture;

#[test]
fn a_refused_edit_is_an_error_and_adds_no_lines() {
    // 2026-09-30 项目主人：只读时写入被拒，和写成了看起来一模一样。被拒的算出错，不算进编辑、不算加减的行数。
    let f = Fixture::new();
    let t0 = Instant::now();
    let mut refused = edit(t0, 1);
    if let StepKind::Tool { state, .. } = &mut refused.kind {
        *state = ToolState::Done(ToolStatus::Denied);
    }
    assert!(refused.failed(), "被拒的算出错：图标换成叉、整行红");
    let seg = segment(vec![edit(t0, 0), refused.clone()], None);
    let got = rows(0, &seg, &f.ctx());
    assert_eq!(text(&got), ["  Used 1 tool · 1 edit +2 -1 · 1 err · 2s"]);
    let err = got[0]
        .line
        .spans
        .iter()
        .find(|s| s.content == "1 err")
        .unwrap();
    assert_eq!(err.style, theme::error(), "出错的那一格红");
    // 只想了一下、写入被拒：照样写出来，不收成 `Thought for`（真模型实测：原来试过写入这件事都看不见了）。
    let seg = segment(vec![thought(t0, 0), refused], None);
    assert_eq!(
        text(&rows(0, &seg, &f.ctx())),
        ["  Used 1 tool · 1 thought · 1 err · 2s"]
    );
}

#[test]
fn a_folded_edit_carries_its_added_and_removed_lines() {
    let f = Fixture::new();
    let t0 = Instant::now();
    let seg = segment(vec![thought(t0, 0), edit(t0, 1)], None);
    let rows = rows(0, &seg, &f.ctx());
    assert_eq!(text(&rows), ["  Made 1 edit +2 -1 · 1 thought · 2s"]);
    let spans = &rows[0].line.spans;
    assert!(
        spans
            .iter()
            .any(|s| s.content == "+2" && s.style == theme::added())
    );
    assert!(
        spans
            .iter()
            .any(|s| s.content == "-1" && s.style == theme::removed())
    );
}

#[test]
fn a_refused_edit_is_red_end_to_end_and_opens_on_why() {
    // 2026-10-13 验收 T-101：只读时被拒的写入，标题后面照参数写绿的 `+1`、红的 `-0`，点开是绿的 `+` 行，看着像写成了。
    let f = Fixture::new();
    let mut refused = edit(Instant::now(), 0);
    if let StepKind::Tool { state, output, .. } = &mut refused.kind {
        *state = ToolState::Done(ToolStatus::Denied);
        *output = "只读：不能写文件".into();
    }
    let style = step::style(&refused, false);
    assert_eq!(style, theme::error());
    let title = step::title(&refused, style, 80, &f.ctx());
    assert!(
        title.iter().all(|s| s.style == theme::error()),
        "整行红，不夹绿字：{title:?}"
    );
    assert!(
        !title.iter().any(|s| s.content.starts_with('+')),
        "没改成，不写加减的行数"
    );
    let body = step::body(&refused, style, 80, &f.ctx());
    let words: Vec<String> = body
        .iter()
        .map(|p| p.content.iter().map(|s| s.content.as_ref()).collect())
        .collect();
    assert_eq!(words, ["只读：不能写文件"], "点开写为什么，不画差异");
    assert!(
        body.iter()
            .all(|p| p.content.iter().all(|s| s.style == theme::error()))
    );
}
