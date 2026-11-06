//! 后台的会话做完了（蓝图「后台命令、子代理和侧边栏」第 7 条）：不是当前这个的会话一轮结束（`busy` 从有到无）记下、标圆点；
//! 别的头开着它的（核心 HV-1 的 `viewed`）由开着的那个通知，这边不标。11-06 CI 那条端到端偶发红查出来的：原来只靠端到端守，
//! 而端到端里那个会话开在另一个终端里，按规矩这边本来就不弹，过不过全看 `viewed` 推得早晚。

use super::app;
use crate::core::{SessionChange, SessionInfo};

fn busy(session: &str) -> SessionInfo {
    SessionInfo {
        session: session.into(),
        busy: true,
        ..SessionInfo::default()
    }
}

#[test]
fn a_background_session_that_finishes_is_marked() {
    let (mut app, _sent) = app();
    // 托盘认领了后台通知：只标圆点、不弹，测试不拉起系统的通知程序（弹的那一步照 `notify/tests.rs`）。
    app.link.notices_claimed = true;
    app.talk.transcript.session = Some("here".into());
    app.talk.sessions_seen = Some(vec![busy("there")]);
    let done = SessionInfo {
        busy: false,
        ..busy("there")
    };
    app.note_finished(&SessionChange::Entry(done));
    assert!(app.talk.unseen.contains("there"));
}

#[test]
fn a_session_open_elsewhere_or_here_or_never_busy_is_not_marked() {
    let (mut app, _sent) = app();
    app.talk.transcript.session = Some("here".into());
    app.talk.sessions_seen = Some(vec![
        busy("there"),
        busy("here"),
        SessionInfo {
            session: "idle".into(),
            ..SessionInfo::default()
        },
    ]);
    let open_elsewhere = SessionInfo {
        busy: false,
        viewed: true,
        ..busy("there")
    };
    app.note_finished(&SessionChange::Entry(open_elsewhere));
    let this_one = SessionInfo {
        busy: false,
        ..busy("here")
    };
    app.note_finished(&SessionChange::Entry(this_one));
    app.note_finished(&SessionChange::Entry(SessionInfo {
        session: "idle".into(),
        ..SessionInfo::default()
    }));
    assert!(app.talk.unseen.is_empty(), "{:?}", app.talk.unseen);
}
