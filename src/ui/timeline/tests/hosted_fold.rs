//! 收起那一行怎么数供应商侧工具（蓝图 `tui.md`「时间线」第 17 条）：和核心视图的 `summary` 同一套算法，照 `timeline.json`
//! 的 `hosted` 表对上的核心工具归类，所以终端和网页写出来是同一句。

use std::time::Instant;

use serde_json::json;

use super::{rows, segment, step, text};
use crate::config::Config;
use crate::core::ToolStatus;
use crate::language::LanguageTable;
use crate::transcript::{Step, StepKind, ToolState};
use crate::ui::test_support::Fixture;

/// 一步工具，从 `start` 秒开始、用了 1 秒；`provider` 不是 `None` 的是供应商侧工具。
fn tool(name: &str, provider: Option<&str>, status: ToolStatus, start: u64) -> Step {
    let mut s = step(
        StepKind::Tool {
            name: name.into(),
            args: String::new(),
            parsed: json!({"queries": ["rust"], "url": "https://a.example", "path": "src"}),
            state: ToolState::Done(status),
            output: "ok".into(),
            said: None,
        },
        Instant::now(),
        start,
        1,
    );
    s.hosted = provider.map(str::to_string);
    s
}

/// DeepSeek 一轮里搜了 2 次、读了 3 个网页，核心另外执行了 1 件 `read`。
fn deepseek_round() -> Vec<Step> {
    let ds = Some("deepseek");
    vec![
        tool("deepseek_web_search", ds, ToolStatus::Ok, 0),
        tool("deepseek_read_url", ds, ToolStatus::Ok, 1),
        tool("deepseek_web_search", ds, ToolStatus::Ok, 2),
        tool("deepseek_read_url", ds, ToolStatus::Ok, 3),
        tool("deepseek_read_url", ds, ToolStatus::Ok, 4),
        tool("read", None, ToolStatus::Ok, 5),
    ]
}

/// 照 `code` 那种语言、自动还是手动，收起那一行。
fn folded(steps: Vec<Step>, code: &str, auto: bool) -> Vec<String> {
    let table = LanguageTable::builtin().unwrap();
    let mut f = Fixture::new();
    f.config = *Config::load(&table.find(code).unwrap(), auto).unwrap();
    text(&rows(0, &segment(steps, None), &f.ctx()))
}

#[test]
fn hosted_searches_and_page_reads_fold_like_the_core_summary() {
    // 同一份数据在网页上照核心的 `summary` 写成这一句（中文、日文照核心的字表）。
    assert_eq!(
        folded(deepseek_round(), "zh", true),
        ["  搜索了 2 次网络 · 3 个网页 · 1 个工具 · 6s"],
        "原来供应商侧工具都按别的工具数，写成 Used 6 tools"
    );
    assert_eq!(
        folded(deepseek_round(), "zh", false),
        ["  搜索了 2 次网络 · 3 个网页 · 1 个工具 · 6s"]
    );
    assert_eq!(
        folded(deepseek_round(), "ja", false),
        ["  ウェブを 2 回検索 · ページ 3 件 · ツール 1 件 · 6s"]
    );
}

#[test]
fn every_hosted_tool_in_the_table_counts_as_its_core_tool() {
    let agy = Some("ag");
    assert_eq!(
        folded(
            vec![
                tool("search_web", agy, ToolStatus::Ok, 0),
                tool("read_url_content", agy, ToolStatus::Ok, 1),
            ],
            "en",
            true,
        ),
        ["  Searched the web 1 time · 1 page · 2s"]
    );
    // 表里没有的算别的工具；名字碰巧和核心的命令一样也不算命令（同核心：供应商侧工具只看 `hosted` 表）。
    for name in ["code_interpreter", "shell"] {
        assert_eq!(
            folded(vec![tool(name, agy, ToolStatus::Ok, 0)], "en", true),
            ["  Used 1 tool · 1s"],
            "{name}"
        );
    }
    // 供应商说失败了的没搜成：算别的工具和一个出错（同核心执行的搜索）。
    assert_eq!(
        folded(
            vec![tool(
                "deepseek_web_search",
                Some("deepseek"),
                ToolStatus::Error,
                0
            )],
            "en",
            true,
        ),
        ["  Used 1 tool · 1 err · 1s"]
    );
    // 中文的「用了」照核心的字表。
    assert_eq!(
        folded(
            vec![tool("code_interpreter", agy, ToolStatus::Ok, 0)],
            "zh",
            false
        ),
        ["  用了 1 个工具 · 1s"]
    );
}
