//! 状态行放不下时往上长（蓝图 `tui.md`「配置页」第 4 条，2026-11-17 项目主人定）：回执、警告、出错的原话折行写全，最多
//! 3 行，再多的最后一行截掉加 `…`；上面几栏照多出来的行让出地方。

use crate::config::Config;
use crate::settings::Tone;

/// 窄窗口（40 列）的配置页，状态行写 `said`：交回屏上的几行。
fn shown(said: &str, tone: Tone) -> Vec<String> {
    let config = Config::builtin().unwrap();
    let mut page = super::page(&config);
    page.say(said.to_string(), tone);
    let (text, _) = super::screen(&mut page, &config, 40, 30);
    text.lines().map(str::to_string).collect()
}

#[test]
fn a_long_warning_grows_upward_and_is_written_in_full() {
    let said = "已保存 · 没有人格 a-very-long-persona-name-that-is-missing-here endmark";
    let lines = shown(said, Tone::Note);
    let at = |word: &str| lines.iter().position(|l| l.contains(word));
    let (Some(top), Some(end)) = (at("已保存"), at("endmark")) else {
        panic!("40 列放不下，折行写全：{lines:#?}");
    };
    assert!(end > top, "往上长：{lines:#?}");
    let rule = crate::sheet::mark(|m| &m.rule);
    assert!(
        lines[end + 1].contains(rule.as_str()),
        "最后一行照旧在细线上面：{lines:#?}"
    );
}

#[test]
fn an_error_longer_than_three_lines_is_cut_with_an_ellipsis() {
    let said: String = (0..40).map(|i| format!("word{i} ")).collect::<String>() + "endmark";
    let lines = shown(&said, Tone::Bad);
    let rows: Vec<&String> = lines.iter().filter(|l| l.contains("word")).collect();
    assert_eq!(rows.len(), 3, "最多 3 行：{lines:#?}");
    assert!(
        rows[2].trim_end().ends_with('…') && !lines.iter().any(|l| l.contains("endmark")),
        "第 3 行截掉加「…」：{lines:#?}"
    );
}
