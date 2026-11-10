//! 给人的通知（`app/notices.rs`，「系统通知」第 8 条）：定时跑完了标题写那个会话的人格（无人格写程序名）、正文写哪个会话跑完了没有，点开切过去。

use base64::Engine;

use crate::core::{Notice, SessionInfo};
use crate::notify::Notifier;

fn b(text: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(text)
}

#[test]
fn a_schedule_run_is_told_under_its_session_and_a_reset_under_miyu() {
    let (mut app, _sent) = crate::app::test_support::app();
    // 照 kitty 认（不看跑测试这台机器的终端）。
    let config = app.config.clone();
    app.notifier = Notifier::new(
        config.notify.clone(),
        config.text.notify.clone(),
        |name| (name == "TERM").then(|| "xterm-kitty".to_string()),
        None,
    );
    app.talk.sessions_seen = Some(vec![SessionInfo {
        session: "s-1".into(),
        preview: Some("看一眼服务器负载，超过 80% 告诉我".into()),
        ..SessionInfo::default()
    }]);
    app.notice(&Notice::ScheduleRan {
        session: "s-1".into(),
        ok: true,
    });
    let out = app.notifier.outbox().concat();
    assert!(
        out.contains(&b("看一眼服务器负载，超过 80% 告诉我：定时任务跑完了")),
        "会话的名字（没起名的照预览）接在正文前面"
    );
    assert!(out.contains(&b("Miyu")) && out.contains("i=miyu-s-s-1:"));
    app.notice(&Notice::ScheduleRan {
        session: "s-2".into(),
        ok: false,
    });
    let out = app.notifier.outbox().concat();
    assert!(
        out.contains(&b("Miyu")) && out.contains(&b("定时任务没跑完")),
        "没见过的会话写 Miyu"
    );
    app.notice(&Notice::SchedulesReset);
    let out = app.notifier.outbox().concat();
    assert!(out.contains(&b("定时任务表坏了，原来的定时都没了")) && !out.contains("report"));
}

/// 后台的会话做完了（「后台命令、子代理和侧边栏」第 7 条，2026-10-30 项目主人：「会话在非聚焦情况下完成，没有任何提示」）：
/// 不是当前这个的一轮结束，记成没看、弹「回答完了」，点了切过去；当前这个的、本来就闲着的不算；切过去就消。
#[test]
fn a_background_session_that_finishes_is_marked_and_told() {
    let (mut app, _sent) = crate::app::test_support::app();
    let config = app.config.clone();
    app.notifier = Notifier::new(
        config.notify.clone(),
        config.text.notify.clone(),
        |name| (name == "TERM").then(|| "xterm-kitty".to_string()),
        None,
    );
    let info = |id: &str, busy: bool| SessionInfo {
        session: id.into(),
        title: Some(format!("会话 {id}")),
        busy,
        ..SessionInfo::default()
    };
    app.talk.transcript.session = Some("me".into());
    app.talk.sessions_seen = Some(vec![
        info("me", true),
        info("bg", true),
        info("idle", false),
    ]);
    for id in ["me", "bg", "idle"] {
        app.session_changed(crate::core::SessionChange::Entry(info(id, false)));
    }
    assert_eq!(
        app.talk.unseen,
        ["bg".to_string()].into(),
        "只有后台在跑、做完的那个"
    );
    let out = app.notifier.outbox().concat();
    assert!(
        out.contains(&b("会话 bg：回答完了")),
        "{out}"
    );
    assert!(out.contains("i=miyu-s-bg:"), "点了切过去");
    assert!(!out.contains(&b("会话 me")), "当前这个照原来那一套");
    app.switch_session("bg".into());
    assert!(app.talk.unseen.is_empty(), "切过去就消");
}

/// 别的头开着的会话（核心 HV-1 的 `viewed`）做完了：由开着的那个通知，这边不标、不弹（2026-11-02 项目主人：herdr 里两个
/// 终端各弹一条「回答完了」）。
#[test]
fn a_session_open_in_another_head_is_left_to_that_head() {
    let (mut app, _sent) = crate::app::test_support::app();
    let config = app.config.clone();
    app.notifier = Notifier::new(
        config.notify.clone(),
        config.text.notify.clone(),
        |name| (name == "TERM").then(|| "xterm-kitty".to_string()),
        None,
    );
    let info = |busy: bool, viewed: bool| SessionInfo {
        session: "other".into(),
        title: Some("别处开着的".into()),
        busy,
        viewed,
        ..SessionInfo::default()
    };
    app.talk.transcript.session = Some("me".into());
    app.talk.sessions_seen = Some(vec![info(true, true)]);
    app.session_changed(crate::core::SessionChange::Entry(info(false, true)));
    assert!(app.talk.unseen.is_empty(), "不标");
    assert!(app.notifier.outbox().concat().is_empty(), "不弹");
}

/// 告诉核心开着哪个会话：变了才说，连上、重连以后再说一遍（核心 HV-1）。
#[test]
fn the_open_session_is_told_once_and_again_after_reconnecting() {
    use crate::core::{Command, Update};
    let (mut app, mut sent) = crate::app::test_support::app();
    let viewing = |sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>| {
        let mut got = Vec::new();
        while let Ok(command) = sent.try_recv() {
            if let Command::Viewing(session) = command {
                got.push(session);
            }
        }
        got
    };
    app.tell_main_session();
    app.tell_main_session();
    assert_eq!(viewing(&mut sent), [None], "空会话：说一次");
    app.talk.transcript.session = Some("s-1".into());
    app.tell_main_session();
    assert_eq!(viewing(&mut sent), [Some("s-1".to_string())]);
    app.core(Update::Reconnected);
    app.tell_main_session();
    assert_eq!(
        viewing(&mut sent),
        [Some("s-1".to_string())],
        "重连以后再说一遍"
    );
}

/// 别的会话在问时弹不弹看它（`ask_elsewhere.rs`）：别的头开着的不弹；自己开着的不算别处。
#[test]
fn a_session_counts_as_open_elsewhere_only_when_another_head_has_it() {
    let (mut app, _sent) = crate::app::test_support::app();
    let item = |id: &str, viewed: bool| SessionInfo {
        session: id.into(),
        viewed,
        ..SessionInfo::default()
    };
    app.talk.sessions_seen = Some(vec![
        item("there", true),
        item("nowhere", false),
        item("mine", true),
    ]);
    app.talk.transcript.session = Some("mine".into());
    app.tell_main_session();
    assert!(app.viewed_elsewhere("there"));
    assert!(!app.viewed_elsewhere("nowhere"));
    assert!(!app.viewed_elsewhere("mine"), "带 viewed 是因为自己开着");
}

/// 核心 TR-1（2026-11-03 项目主人定）：托盘认领了后台通知，没人开着的会话、给人的通知这个头不弹（圆点照画），自己开着的
/// 那一个照弹；认领的走了照原来的弹。
#[test]
fn a_claimed_tray_takes_over_notices_for_sessions_nobody_has_open() {
    let (mut app, _sent) = crate::app::test_support::app();
    let config = app.config.clone();
    app.notifier = Notifier::new(
        config.notify.clone(),
        config.text.notify.clone(),
        |name| (name == "TERM").then(|| "xterm-kitty".to_string()),
        None,
    );
    let info = |id: &str, busy: bool| SessionInfo {
        session: id.into(),
        title: Some(format!("会话 {id}")),
        busy,
        ..SessionInfo::default()
    };
    app.talk.transcript.session = Some("me".into());
    app.talk.sessions_seen = Some(vec![info("me", true), info("bg", true)]);
    app.notice(&Notice::Claimed(true));
    app.session_changed(crate::core::SessionChange::Entry(info("bg", false)));
    assert!(app.talk.unseen.contains("bg"), "圆点照画");
    app.notice(&Notice::ScheduleRan {
        session: "new".into(),
        ok: true,
    });
    app.notice(&Notice::SchedulesReset);
    assert!(app.notifier.outbox().is_empty(), "交给托盘");
    app.notice(&Notice::ScheduleRan {
        session: "me".into(),
        ok: true,
    });
    assert!(!app.notifier.outbox().is_empty(), "自己开着的照弹");
    // 托盘走了：照原来的弹。
    app.notice(&Notice::Claimed(false));
    app.notice(&Notice::SchedulesReset);
    assert!(!app.notifier.outbox().is_empty());
}
