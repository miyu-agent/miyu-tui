//! 系统通知：认终端、弹不弹响不响（蓝图 `tui.md`「系统通知」）。

use std::collections::HashMap;

use super::{Event, Notifier, Plan, Route, Who, plan};
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
    n.tell_notice(
        (Some("早报".into()), None),
        "定时任务跑完了",
        Some("s-1".into()),
    );
    let out = n.outbox().concat();
    assert!(out.contains("i=miyu-s-s-1:"), "{out:?}");
    assert!(out.contains("a=focus,report:"), "{out:?}");
    let b = |t: &str| base64::engine::general_purpose::STANDARD.encode(t);
    // 无人格的会话：标题写程序名，会话标题接在正文前面（2026-11-10）
    assert!(
        out.contains(&b("Miyu")) && out.contains(&b("早报：定时任务跑完了")),
        "{out:?}"
    );
    // 唤醒表坏了的不切会话：标题写 Miyu，不要回话。
    n.tell_notice((None, None), "定时任务表坏了", None);
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
fn the_title_is_the_personas_name_and_the_icon_its_avatar() {
    // 2026-11-10 项目主人：通知照会话的人格来——标题写人格的名字，不写程序名；有头像的带头像，没头像的不带图标；无人格的
    // 会话标题照旧写程序名。会话标题接在正文前面（原来 10-17 定的标题是会话标题）。
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
    let has = |out: &str, part: &str, text: &str| {
        out.contains(&format!("p={part};{}", STANDARD.encode(text)))
    };
    // 无人格：标题写程序名，不带图标（2026-11-10 项目主人：「无人格没有头像」）。
    n.about(Some("修登录页".into()), None);
    n.tell(Event::Replied);
    let out = n.outbox().join("");
    assert!(has(&out, "title", "Miyu"), "{out:?}");
    assert!(has(&out, "body", "修登录页：回答完了"), "{out:?}");
    assert!(!out.contains(":p=icon;"), "无人格不带图标：{out:?}");
    // 有人格、没头像：标题写人格的名字，不带图标。
    let plain = Who {
        name: "小美".into(),
        avatar: None,
    };
    n.about(None, Some(plain));
    n.tell(Event::Asking);
    let out = n.outbox().join("");
    assert!(
        has(&out, "title", "小美") && has(&out, "body", "等你回答"),
        "{out:?}"
    );
    assert!(!out.contains(":p=icon;"), "有人格没头像的不带图标：{out:?}");
    // 有头像：图标是头像缩成的那张。
    std::fs::create_dir_all(&cache).unwrap();
    let face = cache.join("face.png");
    image::RgbImage::new(64, 64).save(&face).unwrap();
    let pictured = Who {
        name: "小美".into(),
        avatar: Some(face),
    };
    n.about(Some("修登录页".into()), Some(pictured.clone()));
    n.tell(Event::Failed);
    let out = n.outbox().join("");
    assert!(
        has(&out, "title", "小美") && out.contains(":p=icon;"),
        "{out:?}"
    );
    // 别的会话来的照传进来的会话写，不照正在看的。
    n.tell_finished(
        (Some("B".into()), Some(&pictured)),
        "回答完了",
        "s-2".into(),
    );
    let out = n.outbox().join("");
    assert!(
        has(&out, "title", "小美") && has(&out, "body", "B：回答完了"),
        "{out:?}"
    );
    drop(std::fs::remove_dir_all(&cache));
}

/// 怎么定标题、正文、图标（纯函数）：名字全是空白的当没有名字，会话标题全是空白的不接。
#[test]
fn shaping_uses_the_name_only_when_it_has_one() {
    use super::{Face, shape};
    let texts = Config::builtin().unwrap().text.notify;
    let blank = Who {
        name: " ".into(),
        avatar: None,
    };
    let (head, body, face) = shape(&texts, (Some("  "), Some(&blank)), "回答完了");
    assert_eq!((head.as_str(), body.as_str()), ("Miyu", "回答完了"));
    assert!(matches!(face, Face::Bare), "有人格没头像的不带图标");
    let (_, _, face) = shape(&texts, (None, None), "回答完了");
    assert!(matches!(face, Face::Bare), "无人格的不带图标");
}

/// 别的会话来的（后台的会话做完了、别的会话在问）在前台也弹、不响（2026-11-02 项目主人：在会话 B 里干活，A 的循环跑完了
/// 没有任何提示）；正在看的会话照旧在前台不弹。
#[test]
fn another_session_pops_up_even_when_focused_but_stays_quiet() {
    let look = Config::builtin().unwrap().notify;
    let here = plan(&look, Route::System, Some(true), false);
    assert_eq!(here.route, None, "正在看的：在前台不弹");
    let there = super::plan_elsewhere(&look, Route::System, Some(true), false);
    assert_eq!((there.route, there.sound), (Some(Route::System), false));
    let away = super::plan_elsewhere(&look, Route::System, Some(false), false);
    assert_eq!(away.sound, look.sound, "离开了照旧响");
    let mut off = look.clone();
    off.enabled = false;
    assert_eq!(
        super::plan_elsewhere(&off, Route::System, Some(true), false).route,
        None
    );
    // kitty 自己判前台：正在看的 `o=unfocused`，别的会话的 `o=always`。
    let config = Config::builtin().unwrap();
    let mut n = Notifier::new(
        config.notify.clone(),
        config.text.notify.clone(),
        env(&[("TERM", "xterm-kitty")]),
        None,
    );
    n.focus(true);
    n.tell(Event::Replied);
    assert!(n.outbox().concat().contains(":o=unfocused:"));
    n.tell_finished((Some("A".into()), None), "回答完了", "s-1".into());
    let out = n.outbox().concat();
    assert!(
        out.contains(":o=always:") && out.contains("report"),
        "{out:?}"
    );
    n.tell_elsewhere(Event::Asking);
    assert!(n.outbox().concat().contains(":o=always:"));
}

/// 发布前第五批审查 T5-4：OSC 9 的字是「标题: 正文」，标题是会话标题（模型起的、别的会话的也会来）。标题照 ConEmu 那一套写成
/// `数字;` 打头时，整句变成 `ESC ] 9 ; 4 ; …`，认 ConEmu 扩展的终端（ghostty 认 `9;4` 进度条）当成子命令，不当通知。
#[test]
fn a_session_title_cannot_turn_osc9_into_a_conemu_subcommand() {
    let config = Config::builtin().unwrap();
    let mut n = Notifier::new(
        config.notify.clone(),
        config.text.notify.clone(),
        env(&[("TERM_PROGRAM", "ghostty")]),
        None,
    );
    // 标题现在是人格的名字（2026-11-10），名字照样可能写成数字打头
    n.about(
        None,
        Some(Who {
            name: "4;3;99".into(),
            avatar: None,
        }),
    );
    n.tell_elsewhere(Event::Asking);
    let out = n.outbox().concat();
    assert!(
        !out.starts_with("\x1b]9;4;"),
        "标题拼进 OSC 9 以后成了 ConEmu 的进度条子命令：{out:?}"
    );
}

/// 发布前第五批审查 T5-5（纵深防御）：kitty 通知的编号 `i=` 直接拼会话编号，不转义、不查字；OSC 99 的别的几格都 base64 了，
/// 这一格是唯一原样进序列的外来字。会话编号现在是核心生成的 UUID，这条测的是「编号里只出现 kitty 认的字」这个前提没人守着。
#[test]
fn a_kitty_notice_id_only_carries_the_characters_kitty_accepts() {
    let config = Config::builtin().unwrap();
    let mut n = Notifier::new(
        config.notify.clone(),
        config.text.notify.clone(),
        env(&[("TERM", "xterm-kitty")]),
        None,
    );
    n.tell_finished(
        (Some("A".into()), None),
        "回答完了",
        "s\x1b\\\x1b]2;x\x07".into(),
    );
    let out = n.outbox().concat();
    let ids: Vec<&str> = out
        .split("\x1b]99;")
        .skip(1)
        .filter_map(|part| part.split([':', ';']).find_map(|kv| kv.strip_prefix("i=")))
        .collect();
    assert!(
        !ids.is_empty()
            && ids.iter().all(|id| id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_+.".contains(c))),
        "编号里混进了 kitty 不认的字（ESC、BEL）：{out:?}"
    );
}
