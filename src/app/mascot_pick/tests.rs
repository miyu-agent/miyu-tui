//! 吉祥物包关着（核心 UX-1补：每个包都有开关，关掉以后它带来的一切都不在）：照内置画、不弹「读不成」；画着的那只的包被关掉，
//! 包开关的推送（`package.changed` 以后的 [`Update::Packages`]）来了再查一次，换回内置的。

use serde_json::json;

use super::{FILE_TAG, LIST_TAG, SETTING_TAG};
use crate::app::test_support::app;
use crate::core::{Command, Update};

fn chose(id: &str) -> Update {
    Update::Answer {
        tag: SETTING_TAG,
        result: Ok(json!({"items": {"tui.mascot": {"value": id}}})),
    }
}

fn listed(enabled: bool) -> Update {
    Update::Answer {
        tag: LIST_TAG,
        result: Ok(json!({"packages": [
            {"package": "pudding", "kind": "mascot", "enabled": enabled, "mascot": {"model": "mascot.json"}}
        ]})),
    }
}

/// 发出去的请求的编号。
fn asked(sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>) -> Vec<u64> {
    std::iter::from_fn(|| sent.try_recv().ok())
        .filter_map(|c| match c {
            Command::Ask { tag, .. } => Some(tag),
            _ => None,
        })
        .collect()
}

#[test]
fn a_switched_off_mascot_package_draws_the_builtin_one_quietly() {
    let (mut app, mut sent) = app();
    app.mascot_pick_update(&chose("pudding"));
    assert!(asked(&mut sent).contains(&LIST_TAG));
    app.mascot_pick_update(&listed(false));
    assert!(!asked(&mut sent).contains(&FILE_TAG), "关着的不去读模型");
    assert_eq!(app.media.mascot_pick.loading, None);
    assert_eq!(app.media.mascot_pick.shown, None);
    assert!(app.typing.notice.is_none(), "不弹「读不成」");
}

#[test]
fn switching_off_the_shown_mascot_package_goes_back_to_the_builtin_one() {
    let (mut app, mut sent) = app();
    let look = app.config.mascot.clone();
    app.put_mascot(look, Some("pudding".into()));
    // 关掉以后核心推 `package.changed`，终端重读软件包列表；配置没变。
    app.mascot_pick_update(&Update::Packages(Vec::new()));
    app.mascot_pick_update(&chose("pudding"));
    assert!(asked(&mut sent).contains(&LIST_TAG), "画着的那只也再查一次");
    app.mascot_pick_update(&listed(false));
    assert_eq!(app.media.mascot_pick.shown, None);
    assert!(app.media.mascot_pick.builtin.is_none(), "换回内置的");
    // 开回来：再查一次，去读模型。
    app.mascot_pick_update(&Update::Packages(Vec::new()));
    app.mascot_pick_update(&chose("pudding"));
    app.mascot_pick_update(&listed(true));
    assert!(asked(&mut sent).contains(&FILE_TAG));
}

#[test]
fn a_package_push_does_not_reread_the_mascot_that_is_still_on() {
    let (mut app, mut sent) = app();
    let look = app.config.mascot.clone();
    app.put_mascot(look, Some("pudding".into()));
    app.mascot_pick_update(&Update::Packages(Vec::new()));
    app.mascot_pick_update(&chose("pudding"));
    app.mascot_pick_update(&listed(true));
    assert!(
        !asked(&mut sent).contains(&FILE_TAG),
        "还开着、画着的就是它：不重读"
    );
    assert_eq!(app.media.mascot_pick.shown.as_deref(), Some("pudding"));
    assert_eq!(app.media.mascot_pick.loading, None);
}
