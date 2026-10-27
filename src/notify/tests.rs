//! 系统通知：认终端、弹不弹响不响（蓝图 `tui.md`「系统通知」）。

use std::collections::HashMap;

use super::{Event, Notifier, Plan, Route, plan};
use crate::config::Config;

fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |name| map.get(name).cloned()
}

#[test]
fn the_route_follows_the_terminal() {
    let osc9 = Config::builtin().unwrap().notify.osc9_programs;
    let route = |pairs: &[(&str, &str)]| Route::detect(&env(pairs), &osc9);
    assert_eq!(route(&[("TERM", "xterm-kitty")]), Route::Kitty);
    assert_eq!(
        route(&[("TERM", "xterm-256color"), ("KITTY_WINDOW_ID", "3")]),
        Route::Kitty
    );
    assert_eq!(route(&[("TERM_PROGRAM", "iTerm.app")]), Route::Osc9);
    assert_eq!(route(&[("TERM_PROGRAM", "ghostty")]), Route::Osc9);
    assert_eq!(route(&[("TERM", "xterm-256color")]), Route::System);
    // tmux 不转 OSC，herdr 吞掉 OSC：交给系统。
    assert_eq!(
        route(&[
            ("TERM", "xterm-kitty"),
            ("TMUX", "/tmp/tmux-1000/default,1,0")
        ]),
        Route::System
    );
    assert_eq!(
        route(&[("TERM", "xterm-kitty"), ("HERDR_ENV", "1")]),
        Route::System
    );
}

#[test]
fn it_only_pops_up_when_away_and_kitty_decides_for_itself() {
    // 2026-09-30 项目主人：终端不在前台时才弹、才响；在 herdr 里由 herdr 来响。
    let look = Config::builtin().unwrap().notify;
    let pop = |route| Plan {
        route: Some(route),
        sound: true,
    };
    assert_eq!(
        plan(&look, Route::System, Some(false), false),
        pop(Route::System)
    );
    assert_eq!(
        plan(&look, Route::System, Some(true), false),
        Plan {
            route: None,
            sound: false
        },
        "在前台：不打扰"
    );
    assert_eq!(
        plan(&look, Route::Osc9, None, false).route,
        None,
        "一次都没报过：当在前台"
    );
    // kitty 自己判前台（o=unfocused），界面只判响不响。
    assert_eq!(
        plan(&look, Route::Kitty, Some(true), false),
        Plan {
            route: Some(Route::Kitty),
            sound: false
        }
    );
    assert!(plan(&look, Route::Kitty, Some(false), false).sound);
    assert!(
        !plan(&look, Route::System, Some(false), true).sound,
        "在 herdr 里不响"
    );
    let mut off = look.clone();
    off.enabled = false;
    assert_eq!(
        plan(&off, Route::Kitty, Some(false), false).route,
        None,
        "关掉整个不弹"
    );
    let mut quiet = look;
    quiet.sound = false;
    assert!(!plan(&quiet, Route::System, Some(false), false).sound);
}

#[test]
fn kitty_gets_its_sequence_in_the_outbox_with_what_happened() {
    let config = Config::builtin().unwrap();
    let mut n = Notifier::new(
        config.notify.clone(),
        config.text.notify.clone(),
        env(&[("TERM", "xterm-kitty")]),
        None,
    );
    n.tell(Event::Replied);
    let out = n.outbox();
    assert_eq!(out.len(), 1);
    assert!(out[0].starts_with("\x1b]99;"), "{:?}", out[0]);
    assert!(n.outbox().is_empty(), "取走就空了");
}

#[test]
fn a_schedule_notice_asks_kitty_to_report_the_click_and_names_its_session() {
    // 核心 J-3 中：每次新开会话的定时跑完了，点开切到那个会话（「系统通知」第 8 条）；平常的通知照旧只跳回窗口。
    use base64::Engine;
    let config = Config::builtin().unwrap();
    let mut n = Notifier::new(
        config.notify.clone(),
        config.text.notify.clone(),
        env(&[("TERM", "xterm-kitty")]),
        None,
    );
    n.tell_notice(Some("早报".into()), "定时任务跑完了", Some("s-1".into()));
    let out = n.outbox().concat();
    assert!(out.contains("i=miyu-s-s-1:"), "{out:?}");
    assert!(out.contains("a=focus,report:"), "{out:?}");
    let b = |t: &str| base64::engine::general_purpose::STANDARD.encode(t);
    assert!(out.contains(&b("早报")) && out.contains(&b("定时任务跑完了")));
    // 唤醒表坏了的不切会话：标题写 Miyu，不要回话。
    n.tell_notice(None, "定时任务表坏了", None);
    let out = n.outbox().concat();
    assert!(
        out.contains("a=focus:") && !out.contains("report"),
        "{out:?}"
    );
    assert!(out.contains(&b("Miyu")));
    n.tell(Event::Replied);
    assert!(
        !n.outbox().concat().contains("report"),
        "平常的通知不要回话"
    );
}

#[test]
fn herdr_keeps_desktop_notifications_and_owns_only_sound() {
    let look = Config::builtin().unwrap().notify;
    assert_eq!(
        plan(&look, Route::System, Some(false), true),
        Plan {
            route: Some(Route::System),
            sound: false
        }
    );
    for focus in [None, Some(true)] {
        assert_eq!(
            plan(&look, Route::System, focus, true),
            Plan {
                route: None,
                sound: false
            }
        );
    }
}

#[test]
fn an_owed_notification_pops_only_when_settled_and_not_after_being_forgiven() {
    // 2026-10-17 项目主人报：长期目标每开一轮续轮都弹一次「已完成」。目标还在跑的先欠着（「系统通知」第 1 条）。
    let config = Config::builtin().unwrap();
    let mut n = Notifier::new(
        config.notify.clone(),
        config.text.notify.clone(),
        env(&[("TERM", "xterm-kitty")]),
        None,
    );
    n.owe(Event::Replied);
    assert!(n.outbox().is_empty(), "欠着不弹");
    n.forgive();
    n.settle();
    assert!(n.outbox().is_empty(), "接着开了一轮：不弹了");
    n.owe(Event::Replied);
    n.settle();
    assert_eq!(n.outbox().len(), 1, "目标停下来：弹一次");
    n.settle();
    assert!(n.outbox().is_empty(), "只弹一次");
}

#[test]
fn the_title_is_the_sessions_and_kitty_gets_an_icon() {
    // 2026-10-17 项目主人定：标题是会话标题，正文只写为什么，图标照人格头像，没有的用 Miyu 的。
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    let config = Config::builtin().unwrap();
    let cache = std::env::temp_dir().join(format!("miyu-notify-about-{}", std::process::id()));
    let mut n = Notifier::new(
        config.notify.clone(),
        config.text.notify.clone(),
        env(&[("TERM", "xterm-kitty")]),
        Some(cache.join("tui").join("sounds")),
    );
    n.about(Some("修登录页".into()), None);
    // 没有吉祥物（还没交下来）的不带图标；交下来了照它画（2026-10-19）。
    n.tell(Event::Replied);
    assert!(
        !n.outbox().join("").contains(":p=icon;"),
        "没交吉祥物的不带图标"
    );
    n.mascot(config.mascot.clone());
    n.tell(Event::Replied);
    let out = n.outbox().join("");
    assert!(
        out.contains(&format!("p=title;{}", STANDARD.encode("修登录页"))),
        "{out:?}"
    );
    assert!(
        out.contains(&format!("p=body;{}", STANDARD.encode("回答完了"))),
        "{out:?}"
    );
    assert!(
        out.contains(":p=icon;"),
        "没有头像用吉祥物画成的图标：{out:?}"
    );
    // 没起名的会话：标题写 Miyu。
    n.about(None, None);
    n.tell(Event::Asking);
    let out = n.outbox().join("");
    assert!(
        out.contains(&format!("p=title;{}", STANDARD.encode("Miyu"))),
        "{out:?}"
    );
    assert!(
        out.contains(&format!("p=body;{}", STANDARD.encode("等你回答"))),
        "{out:?}"
    );
    drop(std::fs::remove_dir_all(&cache));
}

#[test]
fn waiting_notifications_hold_the_laptop_and_finished_ones_stand() {
    // 2026-10-19 项目主人：过程中的通知用玩电脑的，完成的用站着的。
    use super::mascot_icon::Icon;
    assert_eq!(super::pose(Event::Approving), Icon::Working);
    assert_eq!(super::pose(Event::Asking), Icon::Working);
    assert_eq!(super::pose(Event::Replied), Icon::Standing);
    assert_eq!(super::pose(Event::Failed), Icon::Standing);
}
