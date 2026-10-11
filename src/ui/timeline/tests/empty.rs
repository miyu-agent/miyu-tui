//! 还一个字都没有的思考（从 `mod.rs` 挪来，2026-10-11：那个文件过了 500 行）。

use super::{rows, segment};
use crate::transcript::{Step, StepKind};
use crate::ui::test_support::Fixture;

#[test]
fn an_empty_thought_has_only_its_title() {
    // 还一个字都没有的思考不画空的竖线（2026-09-29 项目主人）。
    let f = Fixture::new();
    for text in ["", "  \n "] {
        let mut seg = segment(Vec::new(), None);
        seg.finished = false;
        seg.steps
            .push(Step::new(StepKind::Thought { text: text.into() }));
        let got = super::text(&rows(0, &seg, &f.ctx()));
        assert_eq!(got.len(), 1, "只有标题：{got:?}");
        assert!(got[0].contains("思考中"));
    }
}
