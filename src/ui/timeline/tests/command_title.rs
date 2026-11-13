//! 执行命令那一步的标题（发版前 check 60，2026-11-12 项目主人：「把 timeline 的运行命令前面的 `执行命令 ·` 去掉，直接显示
//! short title」）：有短标题的只写短标题；没写的照旧写工具的显示名。

use std::time::Instant;

use super::{command, segment, text};
use crate::core::ToolStatus;
use crate::transcript::StepKind;
use crate::ui::test_support::Fixture;
use crate::ui::timeline::rows;

#[test]
fn a_command_with_a_short_title_shows_only_the_title() {
    let f = Fixture::new();
    let t0 = Instant::now();
    let mut bare = command(t0, 2, ToolStatus::Ok);
    if let StepKind::Tool { parsed, .. } = &mut bare.kind {
        *parsed = serde_json::json!({"command": "ls -la"});
    }
    let seg = segment(vec![command(t0, 1, ToolStatus::Ok), bare], Some(true));
    let lines = text(&rows(0, &seg, &f.ctx()));
    let titles: Vec<&str> = lines
        .iter()
        .map(|l| l.trim_start())
        .filter(|l| !l.starts_with('│') && !l.is_empty())
        .skip(1)
        .collect();
    assert!(titles[0].ends_with(" 列目录"), "只写短标题：{lines:?}");
    assert!(
        !titles[0].contains(" · "),
        "不再「名字 · 短标题」：{lines:?}"
    );
    assert!(
        titles[1].ends_with(" shell"),
        "没写短标题的照旧写名字：{lines:?}"
    );
}
