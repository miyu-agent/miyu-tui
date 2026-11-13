//! `/rename`（蓝图「改名」）：带名字的改名，只打 `/rename` 让模型重起一个（核心 TT-1，2026-11-11 项目主人：原来是去掉标题）。

use crate::core::{Command, Update};
use crate::input::Draft;

/// 发给核心的命令。
fn drained(sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>) -> Vec<Command> {
    std::iter::from_fn(|| sent.try_recv().ok()).collect()
}

/// 弹着的那一句提示。
fn notice(app: &crate::app::App) -> Option<&str> {
    app.typing.notice.as_ref().map(|n| n.text.as_str())
}

#[test]
fn a_bare_rename_asks_the_model_and_one_with_words_sets_the_title() {
    let (mut app, mut sent) = crate::app::test_support::app();
    app.talk.transcript.session = Some("s1".into());
    drained(&mut sent);
    app.submit(Draft::plain("/rename"));
    let got = drained(&mut sent);
    assert!(
        matches!(got.as_slice(), [Command::Retitle]),
        "只打 /rename 让模型重起，不再把标题写成空的：{got:?}"
    );
    app.submit(Draft::plain("/rename    "));
    assert!(
        matches!(drained(&mut sent).as_slice(), [Command::Retitle]),
        "后面只有空白的也一样"
    );
    app.submit(Draft::plain("/rename   回文函数  "));
    let got = drained(&mut sent);
    assert!(
        matches!(got.as_slice(), [Command::Rename(Some(title))] if title == "回文函数"),
        "{got:?}"
    );
    // 只写一个 `-`：去掉标题（2026-11-12 项目主人）。
    app.submit(Draft::plain("/rename -"));
    let got = drained(&mut sent);
    assert!(matches!(got.as_slice(), [Command::Rename(None)]), "{got:?}");
    // 太长的当场提示、不发。
    app.submit(Draft::plain(&format!("/rename {}", "长".repeat(201))));
    assert!(drained(&mut sent).is_empty());
    assert_eq!(notice(&app), Some("标题最多 200 个字"));
}

#[test]
fn the_reply_says_it_is_retitling_and_nothing_to_title_says_so() {
    let (mut app, _sent) = crate::app::test_support::app();
    app.talk.transcript.session = Some("s1".into());
    app.core(Update::Retitling);
    assert_eq!(notice(&app), Some("正在重新起标题…"));
    // 她一句带正文的回答都还没有：核心回 `bad_params`，连接那头换成 `nothing_to_retitle`（`core/awaiting.rs`）。
    app.core(Update::Refused {
        reason: Some("nothing_to_retitle".into()),
        message: "参数不对。".into(),
    });
    assert_eq!(notice(&app), Some("还没有可以起标题的对话"));
    assert!(
        app.talk.transcript.entries.is_empty(),
        "只弹提示，不写进正文"
    );
    app.core(Update::Renamed(Some("回文函数".into())));
    assert_eq!(notice(&app), Some("已改名：回文函数"));
    app.core(Update::Renamed(None));
    assert_eq!(notice(&app), Some("已清除标题"));
}

/// 核心 TT-3（2026-11-13）：不带字的 `/rename` 没起成（超时、出错、模型要了工具），推 `session.retitle_failed`：说核心那一句
/// （`human` 照 `human.get` 换字），后面接那一次的错；换不出字的照终端自己的「更改标题失败」。
#[test]
fn a_failed_retitle_says_so_with_the_error() {
    let (mut app, _sent) = crate::app::test_support::app();
    app.talk.transcript.session = Some("s1".into());
    let failed = |said| {
        Update::Push(crate::core::Push::RetitleFailed {
            said,
            error: "timed out".into(),
        })
    };
    app.core(failed(None));
    assert_eq!(notice(&app), Some("更改标题失败"));
    let tail = app.typing.notice.as_ref().map(|n| n.tail.clone());
    assert!(tail.is_some_and(|t| t.ends_with("timed out")));
    app.link.human = crate::human::Human::from_reply(&serde_json::json!({
        "language": "zh",
        "said": {"core/sessions/retitle-failed": "标题没改成"}
    }));
    let said =
        serde_json::from_value(serde_json::json!({"key": "core/sessions/retitle-failed"})).ok();
    app.core(failed(said));
    assert_eq!(notice(&app), Some("标题没改成"), "照核心的字");
}
