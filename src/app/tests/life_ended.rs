//! 一段人生结束了（「推演人生」第 9 条，核心 LF-14）：她自己结束的弹系统通知「美羽 的人生结束了」，人结束的不弹；没看过的在
//! 输入框上面那一块最上面写一行，点「收起」调 `life.seen`，推来 `life.seen` 收起。

use base64::Engine;
use ratatui::layout::{Position, Rect};
use serde_json::json;

use super::app;
use crate::core::{Command, Notice, SessionInfo, Update};
use crate::notify::Notifier;

fn b(text: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(text)
}

fn asked(sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>) -> Vec<String> {
    let mut got = Vec::new();
    while let Ok(command) = sent.try_recv() {
        if let Command::Ask { method, params, .. } = command {
            got.push(format!("{method} {params}"));
        }
    }
    got
}

fn kitty(app: &mut crate::app::App) {
    let config = app.config.clone();
    app.notifier = Notifier::new(
        config.notify.clone(),
        config.text.notify.clone(),
        |name| (name == "TERM").then(|| "xterm-kitty".to_string()),
        None,
    );
}

fn ended(by_self: bool) -> Notice {
    Notice::LifeEnded {
        session: "s-life".into(),
        name: Some("美羽".into()),
        by_self,
    }
}

#[test]
fn only_a_life_she_ended_herself_is_told_and_clicking_opens_it() {
    let (mut app, mut sent) = app();
    kitty(&mut app);
    app.talk.sessions_seen = Some(vec![SessionInfo {
        session: "s-life".into(),
        persona: Some("miyu".into()),
        ..SessionInfo::default()
    }]);
    app.notice(&ended(false));
    assert!(app.notifier.outbox().concat().is_empty(), "人结束的不弹");
    assert_eq!(asked(&mut sent), ["life.list {}"], "照样重读没看过的");
    app.notice(&ended(true));
    let out = app.notifier.outbox().concat();
    assert!(out.contains(&b("美羽 的人生结束了")), "{out}");
    assert!(out.contains("i=miyu-s-s-life:"), "点开切到那个会话");
    // 有人认领了后台通知（托盘开着）、没开着那个会话的：由认领的那一头弹。
    app.notice(&Notice::Claimed(true));
    let _ = app.notifier.outbox();
    app.notice(&ended(true));
    assert!(app.notifier.outbox().concat().is_empty());
}

#[test]
fn an_unseen_end_stays_on_top_until_dismissed_or_seen_elsewhere() {
    let (mut app, mut sent) = app();
    // 连上核心（订 `notices` 的回应）时问一次。
    app.notice(&Notice::Claimed(false));
    assert_eq!(asked(&mut sent), ["life.list {}"]);
    app.core(Update::Answer {
        tag: crate::oobe::asker::TAG_BASE - 52,
        result: Ok(json!({"lives": [], "ended_unseen": [
            {"session": "s-a", "persona": "miyu", "name": "美羽", "at": "2026-11-17T08:00:00.000Z"},
            {"session": "s-b", "persona": "luna", "name": "露娜", "at": "2026-11-17T09:00:00.000Z"}
        ]})),
    });
    let top = crate::ui::above_lines(&app, 60);
    assert_eq!(
        top[0].to_string(),
        "美羽 的人生结束了（还有 1 段）  打开  收起"
    );
    // 点「收起」：调 life.seen，当场收起，下一段顶上来。
    app.screen.areas.todo = Rect::new(4, 30, 60, 3);
    let first = app.link.ended_unseen[0].clone();
    let (_, [_, (dismiss, _)]) =
        crate::app::life_ended::ended_line(&first, 1, &app.config.text.loops.life);
    // 「收起」从第 `dismiss` 列起（两个中文字四列）：点在它第二个字上。
    let x = 4 + u16::try_from(dismiss).unwrap() + 2;
    assert!(app.ended_clicked(Position::new(x, 30)));
    assert_eq!(asked(&mut sent), [r#"life.seen {"session":"s-a"}"#]);
    assert_eq!(
        crate::ui::above_lines(&app, 60)[0].to_string(),
        "露娜 的人生结束了  打开  收起"
    );
    // 别的头看过了：推来 life.seen，收起。
    app.notice(&Notice::LifeSeen {
        session: "s-b".into(),
    });
    assert!(app.link.ended_unseen.is_empty());
    assert!(crate::ui::above_lines(&app, 60).is_empty());
}
