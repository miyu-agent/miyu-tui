//! 框下面那一行用量那一格（蓝图 `tui.md`「框下面那一行」用量那一格，反馈 1102 第 22 条）。

use super::{Facts, Hit, Piece, pieces, ring};
use crate::config::Config;
use crate::core::{CallError, Compaction, Limits, Push, Update, Usage};
use crate::transcript::Transcript;

fn text(pieces: &[Piece]) -> Vec<String> {
    pieces
        .iter()
        .map(|p| p.spans.iter().map(|s| s.content.as_ref()).collect())
        .collect()
}

fn lay(t: &Transcript, total: Usage, spent: Option<&str>, config: &Config) -> Vec<String> {
    let facts = Facts {
        t,
        preview: None,
        total,
        spent: spent.map(str::to_string),
        texts: &config.text.usage_cell,
        spinner: &config.timeline.spinner,
    };
    text(&pieces(&facts, 0, None))
}

fn push(t: &mut Transcript, push: Push, config: &Config) {
    t.update(Update::Push(push), &config.text);
}

fn used(tokens: u64) -> Usage {
    Usage {
        uncached: tokens,
        ..Usage::default()
    }
}

#[test]
fn the_ring_fills_by_eighths_of_the_window() {
    let window = Some(800);
    let rings: Vec<String> = [0, 99, 100, 299, 300, 499, 500, 699, 700, 900]
        .iter()
        .map(|&u| ring(u, window))
        .collect();
    assert_eq!(rings, ["○", "○", "◔", "◔", "◑", "◑", "◕", "◕", "●", "●"]);
    assert_eq!(ring(500, None), "○", "没窗口的画空环");
}

#[test]
fn ring_tokens_and_money_with_nothing_before_anything_is_used() {
    let config = Config::builtin().unwrap();
    let mut t = Transcript::default();
    assert!(
        lay(&t, Usage::default(), None, &config).is_empty(),
        "全是 0 不写"
    );
    t.context = 300_000;
    t.limits = Limits {
        window: Some(1_000_000),
        compaction_line: Some(800_000),
        context: None,
    };
    // 累计连子代理由调用的一方给（`Talk::usage_total`），这里给多少写多少。
    assert_eq!(
        lay(&t, used(128_400), Some("¥0.42"), &config),
        ["◔ 128.4k(C0%)", "¥0.42"]
    );
    assert_eq!(
        lay(&t, used(128_400), None, &config),
        ["◔ 128.4k(C0%)"],
        "算不出钱的不写"
    );
}

#[test]
fn a_new_session_shows_the_previewed_ring_only() {
    let config = Config::builtin().unwrap();
    let t = Transcript::default();
    let facts = Facts {
        t: &t,
        preview: Some((342_600, Some(1_000_000))),
        total: used(5),
        spent: Some("¥0.01".into()),
        texts: &config.text.usage_cell,
        spinner: &config.timeline.spinner,
    };
    assert_eq!(text(&pieces(&facts, 0, None)), ["◔"]);
}

#[test]
fn a_failed_compaction_past_the_line_turns_red_with_a_compact_button_until_it_compacts() {
    let config = Config::builtin().unwrap();
    let mut t = Transcript::default();
    t.limits = Limits {
        window: Some(256_000),
        compaction_line: Some(190_000),
        context: None,
    };
    t.context = 203_000;
    assert_eq!(
        lay(&t, used(1), None, &config)[0],
        "◕ 1(C0%)",
        "失败以前只是到了线，不算卡住"
    );
    let failed = CallError {
        class: "timeout".into(),
        message: "timed out".into(),
        status: None,
        retry_after_ms: None,
    };
    push(
        &mut t,
        Push::Compaction(Compaction::Failed(failed)),
        &config,
    );
    let facts = Facts {
        t: &t,
        preview: None,
        total: used(1),
        spent: None,
        texts: &config.text.usage_cell,
        spinner: &config.timeline.spinner,
    };
    let laid = pieces(&facts, 0, None);
    assert_eq!(text(&laid), ["◕ 上下文压缩失败", "203k/190k", "压缩"]);
    assert_eq!(laid[2].hit, Some(Hit::Compact));
    assert!(laid[1].tight && laid[2].tight, "几截之间一个空格");
    assert_eq!(laid[0].spans[0].style.fg, crate::theme::error().fg, "红字");
    // 压好了：回到平常。
    let done = Compaction::Done {
        before: 203_000,
        after: 31_000,
        prepared: false,
    };
    push(&mut t, Push::Compaction(done), &config);
    t.context = 31_000;
    assert_eq!(lay(&t, used(1), None, &config), ["○ 1(C0%)"]);
}

#[test]
fn a_paused_compaction_below_the_line_is_not_stuck_and_over_the_window_is() {
    let config = Config::builtin().unwrap();
    let mut t = Transcript::default();
    t.limits = Limits {
        window: Some(128_000),
        compaction_line: Some(100_000),
        context: None,
    };
    t.context = 50_000;
    let paused = Compaction::Paused {
        reason: "failures".into(),
        failures: Some(3),
        entry: None,
    };
    push(&mut t, Push::Compaction(paused), &config);
    assert_eq!(
        lay(&t, used(1), None, &config),
        ["◑ 1(C0%)"],
        "没到线不算卡住"
    );
    // 换到了窗口小的模型：用量大于窗口。暂停着、也到了线的照压缩失败写。
    t.context = 203_000;
    assert_eq!(lay(&t, used(1), None, &config)[0], "● 上下文压缩失败");
    push(&mut t, Push::Compacted { clear: true }, &config);
    t.context = 203_000;
    assert_eq!(
        lay(&t, used(1), None, &config),
        ["● 上下文超出窗口", "203k/128k", "压缩"],
        "清空过不再算压缩失败，超出窗口照写"
    );
}

#[test]
fn compacting_shows_a_spinner_instead() {
    let config = Config::builtin().unwrap();
    let mut t = Transcript::default();
    t.context = 203_000;
    let progress = Compaction::Progress {
        written: 10,
        expected: None,
        manual: Some(true),
    };
    push(&mut t, Push::Compaction(progress), &config);
    let laid = lay(&t, used(1), None, &config);
    assert_eq!(laid.len(), 1);
    assert!(laid[0].ends_with(" 正在压缩上下文"), "{laid:?}");
    assert!(laid[0].starts_with(&config.timeline.spinner[0]));
}

/// 设了会话的上下文上限（核心 CTX-1，设计 68）：环和累计的左边写「会话上限 128k」，人知道窗口是被它压小的；没设的不写。
#[test]
fn a_session_cap_is_named_before_the_ring() {
    let config = Config::builtin().unwrap();
    let mut t = Transcript::default();
    t.context = 30_000;
    t.limits = Limits {
        window: Some(128_000),
        compaction_line: Some(101_000),
        context: Some(128_000),
    };
    assert_eq!(
        lay(&t, used(5_000), Some("¥0.42"), &config),
        ["会话上限 128k", "◔ 5k(C0%)", "¥0.42"]
    );
}

/// 累计后面紧跟这个会话的缓存命中率（2026-11-10 项目主人：「xxxM(C99%)」）：命中 ÷ 输入，输出不算；只有辅助请求、没记下
/// 输入的不写。
#[test]
fn the_total_is_followed_by_the_cache_hit_rate() {
    let config = Config::builtin().unwrap();
    let mut t = Transcript::default();
    t.context = 30_000;
    t.limits = Limits {
        window: Some(128_000),
        compaction_line: Some(101_000),
        context: None,
    };
    let total = Usage {
        uncached: 1_000,
        cache_read: 99_000,
        output: 5_000,
        ..Usage::default()
    };
    assert_eq!(lay(&t, total, None, &config), ["◔ 105k(C99%)"]);
    let aux = Usage {
        aux: 2_000,
        ..Usage::default()
    };
    assert_eq!(lay(&t, aux, None, &config), ["◔ 2k"]);
}
