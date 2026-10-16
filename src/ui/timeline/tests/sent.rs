//! 只为发一张图的那一段（核心 X-24，蓝图 `tui.md`「时间线」第 17 条，2026-10-16 项目主人定）：收起那一行打头写发出那张的
//! 那件工具的显示名，不写 `Used N tools`。

use std::time::Instant;

use serde_json::json;

use super::{rows, segment, step, text, thought};
use crate::config::Config;
use crate::core::ToolStatus;
use crate::human::Human;
use crate::language::LanguageTable;
use crate::transcript::{Step, StepKind, ToolState};
use crate::ui::test_support::Fixture;

/// 一步工具：名字 `name`，做成了，发出去了 `sent` 张；调用那一刻记下了中英两种显示名（`meme_send` 的）。
fn tool(name: &str, sent: usize, t0: Instant, start: u64) -> Step {
    let mut step = step(
        StepKind::Tool {
            name: name.into(),
            args: String::new(),
            parsed: json!({}),
            state: ToolState::Done(ToolStatus::Ok),
            output: String::new(),
            said: None,
        },
        t0,
        start,
        1,
    );
    step.sent = sent;
    if name == "meme_send" {
        step.display = [
            ("en".to_string(), "Send a sticker".to_string()),
            ("zh".to_string(), "发送表情包".to_string()),
        ]
        .into();
    }
    step
}

/// 中文界面、自动（收起那一行英文）。表情包的两件工具属于 `miyu-memes`，`read` 属于出厂的包，`fetch` 属于别的包，
/// `bare` 不在目录里。
fn fixture() -> Fixture {
    let mut f = Fixture::new();
    f.human = Human::from_reply(&json!({"language": "zh", "said": {},
        "tools": {
            "meme_search": {"name": "搜索表情包"},
            "meme_send": {"name": "发送表情包"},
            "read": {"name": "读取"},
            "fetch": {"name": "抓取"},
            "bare": {"name": "无包"}
        },
        "packages": {"meme_search": "miyu-memes", "meme_send": "miyu-memes", "read": "basesystem", "fetch": "net"}
    }));
    f
}

#[test]
fn a_segment_that_only_sends_one_sticker_is_named_by_the_sending_tool() {
    let f = fixture();
    let t0 = Instant::now();
    let fold = |steps| text(&rows(0, &segment(steps, None), &f.ctx()));
    assert_eq!(
        fold(vec![tool("meme_send", 1, t0, 0)]),
        ["  Send a sticker · 1s"],
        "自动时整行英文：显示名也取英文那一份（2026-10-16 项目主人验 T-260）"
    );
    assert_eq!(
        fold(vec![
            thought(t0, 0),
            tool("meme_search", 0, t0, 1),
            tool("meme_send", 1, t0, 2)
        ]),
        ["  Send a sticker · 1 thought · 3s"],
        "先搜再发也算，想过的照旧接在后面"
    );
    let mut unnamed = tool("meme_send", 1, t0, 0);
    unnamed.display.clear();
    assert_eq!(
        fold(vec![unnamed]),
        ["  meme_send · 1s"],
        "没记下英文的写工具名，不拿界面语言的工具表凑"
    );
}

#[test]
fn a_chosen_language_names_the_sending_tool_in_that_language() {
    let mut f = fixture();
    let table = LanguageTable::builtin().unwrap();
    f.config = Config::load(&table.find("zh").unwrap(), false).unwrap();
    let t0 = Instant::now();
    let fold = |steps| text(&rows(0, &segment(steps, None), &f.ctx()));
    assert_eq!(
        fold(vec![
            thought(t0, 0),
            thought(t0, 1),
            tool("meme_send", 1, t0, 2)
        ]),
        ["  发送表情包 · 2 次思考 · 3s"],
        "手动选了中文：整行中文"
    );
    let mut unnamed = tool("meme_send", 1, t0, 0);
    unnamed.display.clear();
    assert_eq!(
        fold(vec![unnamed]),
        ["  发送表情包 · 1s"],
        "没记下的照这一刻的工具表"
    );
}

#[test]
fn other_packages_more_pictures_or_no_package_still_count_tools() {
    let f = fixture();
    let t0 = Instant::now();
    let fold = |steps| text(&rows(0, &segment(steps, None), &f.ctx()));
    assert_eq!(
        fold(vec![tool("read", 0, t0, 0), tool("meme_send", 1, t0, 1)]),
        ["  Used 2 tools · 2s"],
        "混了别的包的工具"
    );
    assert_eq!(
        fold(vec![
            tool("meme_send", 1, t0, 0),
            tool("meme_send", 1, t0, 1)
        ]),
        ["  Used 2 tools · 2s"],
        "发了不止一张"
    );
    assert_eq!(
        fold(vec![tool("meme_search", 0, t0, 0)]),
        ["  Used 1 tool · 1s"],
        "一张都没发"
    );
    assert_eq!(
        fold(vec![tool("bare", 1, t0, 0)]),
        ["  Used 1 tool · 1s"],
        "没有包编号的不算"
    );
    assert_eq!(
        fold(vec![tool("fetch", 0, t0, 0), tool("meme_send", 1, t0, 1)]),
        ["  Used 2 tools · 2s"]
    );
}
