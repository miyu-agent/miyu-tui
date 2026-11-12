//! `/compact-line`（核心 CT-1）：发什么、回执怎么说、只打命令说哪一种。

use serde_json::json;

use super::{Asked, LINE_TAG, SET_TAG, read};
use crate::core::{Command, Push, Refusal, Update};
use crate::input::Draft;

/// 发给核心的命令。
fn drained(sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>) -> Vec<Command> {
    std::iter::from_fn(|| sent.try_recv().ok()).collect()
}

/// 弹着的那一句提示。
fn notice(app: &crate::app::App) -> Option<&str> {
    app.typing.notice.as_ref().map(|n| n.text.as_str())
}

/// 发出去的 `session.configure` 的 `compaction_line`；没有发的是 `None`。
fn configured(got: &[Command]) -> Option<serde_json::Value> {
    got.iter().find_map(|c| match c {
        Command::Ask {
            tag,
            method: "session.configure",
            params,
        } if *tag == SET_TAG => Some(params["compaction_line"].clone()),
        _ => None,
    })
}

#[test]
fn the_words_after_the_command_are_read_as_a_percent_or_off() {
    assert_eq!(read(None), Asked::Show);
    assert_eq!(read(Some("   ")), Asked::Show);
    assert_eq!(read(Some("70")), Asked::Set(Some(70)));
    assert_eq!(read(Some("70%")), Asked::Set(Some(70)));
    assert_eq!(read(Some(" OFF ")), Asked::Set(None));
    for bad in ["19", "96", "abc", "7.5", "-1"] {
        assert_eq!(read(Some(bad)), Asked::Bad, "{bad}");
    }
    assert_eq!(read(Some("20")), Asked::Set(Some(20)));
    assert_eq!(read(Some("95")), Asked::Set(Some(95)));
}

#[test]
fn a_percent_and_off_are_sent_and_the_receipt_says_what_it_became() {
    let (mut app, mut sent) = crate::app::test_support::app();
    app.talk.transcript.session = Some("s1".into());
    app.talk.compaction_line = Some(85);
    drained(&mut sent);
    app.submit(Draft::plain("/compact-line 70"));
    assert_eq!(configured(&drained(&mut sent)), Some(json!(70)));
    app.core(Update::Answer {
        tag: SET_TAG,
        result: Ok(json!({})),
    });
    assert_eq!(notice(&app), Some("这个会话的压缩线改成 70%"));
    assert_eq!(app.talk.transcript.compaction_pin, Some(70));
    app.submit(Draft::plain("/compact-line off"));
    assert_eq!(configured(&drained(&mut sent)), Some(json!(null)));
    app.core(Update::Answer {
        tag: SET_TAG,
        result: Ok(json!({})),
    });
    assert_eq!(notice(&app), Some("压缩线回到跟着设置（85%）"));
    assert_eq!(app.talk.transcript.compaction_pin, None);
}

#[test]
fn out_of_range_is_said_here_and_when_the_core_refuses() {
    let (mut app, mut sent) = crate::app::test_support::app();
    app.talk.transcript.session = Some("s1".into());
    drained(&mut sent);
    app.submit(Draft::plain("/compact-line 99"));
    assert!(configured(&drained(&mut sent)).is_none(), "不在范围的不发");
    assert_eq!(notice(&app), Some("压缩线要在 20 到 95 之间"));
    // 核心照样拒的（`bad_params`，`data.field` 是 `compaction_line`）说同一句；别的拒绝照原话。
    app.submit(Draft::plain("/compact-line 50"));
    drained(&mut sent);
    app.core(Update::Answer {
        tag: SET_TAG,
        result: Err(Refusal {
            reason: Some("bad_params".into()),
            message: "参数不对。".into(),
            data: json!({"field": "compaction_line"}),
        }),
    });
    assert_eq!(notice(&app), Some("压缩线要在 20 到 95 之间"));
    app.submit(Draft::plain("/compact-line 50"));
    app.core(Update::Answer {
        tag: SET_TAG,
        result: Err(Refusal {
            reason: Some("session_not_found".into()),
            message: "没有这个会话。".into(),
            data: json!(null),
        }),
    });
    assert_eq!(notice(&app), Some("没有这个会话"));
}

#[test]
fn the_bare_command_says_the_line_and_where_it_comes_from() {
    let (mut app, mut sent) = crate::app::test_support::app();
    app.talk.transcript.session = Some("s1".into());
    drained(&mut sent);
    app.submit(Draft::plain("/compact-line"));
    assert!(configured(&drained(&mut sent)).is_none(), "只打命令不发");
    assert_eq!(notice(&app), Some("压缩线跟着设置"), "配置还没读到：不写数");
    app.core(Update::Answer {
        tag: LINE_TAG,
        result: Ok(json!({"items": {"compaction.line": {"value": 85}}})),
    });
    app.submit(Draft::plain("/compact-line"));
    assert_eq!(notice(&app), Some("压缩线 85%（跟着设置）"));
    // 推来的 `session.policy_changed` 钉了（别的头、im 网页改的也算）。
    app.talk.transcript.update(
        Update::Push(Push::CompactionLineSet(Some(60))),
        &app.config.text.clone(),
    );
    app.submit(Draft::plain("/compact-line"));
    assert_eq!(notice(&app), Some("压缩线 60%（这个会话）"));
}

#[test]
fn a_session_not_opened_yet_is_told_and_nothing_is_sent() {
    let (mut app, mut sent) = crate::app::test_support::app();
    drained(&mut sent);
    app.submit(Draft::plain("/compact-line 70"));
    assert!(configured(&drained(&mut sent)).is_none());
    assert_eq!(notice(&app), Some("会话尚未创建，无法设压缩线"));
}
