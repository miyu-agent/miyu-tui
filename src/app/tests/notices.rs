//! 给人的通知（`app/notices.rs`，「系统通知」第 8 条）：定时跑完了标题写那个会话、正文写跑完了没有，点开切过去。

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
        out.contains(&b("看一眼服务器负载，超过 80% 告诉我")),
        "没起名的照预览"
    );
    assert!(out.contains(&b("定时任务跑完了")) && out.contains("i=miyu-s-s-1:"));
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
