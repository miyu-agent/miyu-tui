//! 循环开的一轮那一行（蓝图 `tui.md`「循环」第 5 条）：打头的钟照图标那一套（2026-11-20 主人本机验收：用的是 `nerd`，
//! 这一行却画 `plain` 的 `◷`，小得看不清）。

use crate::core::{Push, Update};
use crate::transcript::Transcript;
use crate::ui::test_support::Fixture;

/// 换成叫 `name` 的那一套图标。
fn use_icons(f: &mut Fixture, name: &str) {
    let set = f.config.icon_sets.iter().find(|s| s.name == name);
    f.config.icons = set.cloned().expect("出厂有这一套");
}

/// 循环交进来的那一句排成的第一行。
fn round_line(f: &Fixture) -> String {
    let mut t = Transcript::default();
    t.update(
        Update::Push(Push::Wake {
            seq: 30,
            words: None,
        }),
        &f.config.text,
    );
    let rows = super::entry_rows(0, &t.entries[0], &f.ctx());
    rows[0].line.to_string()
}

#[test]
fn the_loop_round_line_leads_with_the_icon_sets_clock() {
    let mut f = Fixture::new();
    use_icons(&mut f, "nerd");
    let line = round_line(&f);
    assert!(
        line.trim_start()
            .starts_with("\u{f017} 循环 · 由 AI 自己决定"),
        "nerd 那一套是 Font Awesome 的钟：{line:?}"
    );
    use_icons(&mut f, "plain");
    let line = round_line(&f);
    assert!(
        line.trim_start().starts_with("◷ 循环 · 由 AI 自己决定"),
        "plain 照旧：{line:?}"
    );
}
