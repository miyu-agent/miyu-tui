//! 只为发一张图的那一段（核心 X-24，蓝图 `tui.md`「时间线」第 17 条，2026-10-16 项目主人定）：收起那一行打头写发出那张的
//! 那件工具的显示名，不写 `Used N tools`。

use std::time::Instant;

use serde_json::json;

use super::{rows, segment, step, text, thought};
use crate::core::ToolStatus;
use crate::human::Human;
use crate::transcript::{Step, StepKind, ToolState};
use crate::ui::test_support::Fixture;

/// 一步工具：名字 `name`，做成了，发出去了 `sent` 张。
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
    step
}

/// 表情包的两件工具属于 `miyu-memes`，`read` 属于出厂的包，`fetch` 属于别的包，`bare` 不在目录里。
fn fixture() -> Fixture {
    let mut f = Fixture::new();
    f.human = Human::from_reply(&json!({"language": "en", "said": {},
        "tools": {
            "meme_search": {"name": "Search stickers"},
            "meme_send": {"name": "Send sticker"},
            "read": {"name": "Read"},
            "fetch": {"name": "Fetch"},
            "bare": {"name": "Bare"}
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
        ["  Send sticker · 1s"]
    );
    assert_eq!(
        fold(vec![
            thought(t0, 0),
            tool("meme_search", 0, t0, 1),
            tool("meme_send", 1, t0, 2)
        ]),
        ["  Send sticker · 1 thought · 3s"],
        "先搜再发也算，想过的照旧接在后面"
    );
    let mut named = tool("meme_send", 1, t0, 0);
    named.display = [("en".to_string(), "Send a meme".to_string())].into();
    assert_eq!(
        fold(vec![named]),
        ["  Send a meme · 1s"],
        "照调用那一刻记下的显示名"
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
