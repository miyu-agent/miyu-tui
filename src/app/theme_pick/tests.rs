//! 主题包关着（核心 UX-1补）：照默认主题画、不弹「读不成」；包开关的推送来了再查一次。

use serde_json::json;

use super::{FILE_TAG, LIST_TAG, SETTING_TAG};
use crate::app::test_support::app;
use crate::core::{Command, Update};

fn chose(id: &str) -> Update {
    Update::Answer {
        tag: SETTING_TAG,
        result: Ok(json!({"items": {"tui.look": {"value": id}}})),
    }
}

fn listed(enabled: bool) -> Update {
    Update::Answer {
        tag: LIST_TAG,
        result: Ok(json!({"packages": [
            {"package": "night", "kind": "theme", "enabled": enabled, "theme": {"tui": {"file": "theme.json"}}}
        ]})),
    }
}

fn asked(sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>) -> Vec<u64> {
    std::iter::from_fn(|| sent.try_recv().ok())
        .filter_map(|c| match c {
            Command::Ask { tag, .. } => Some(tag),
            _ => None,
        })
        .collect()
}

#[test]
fn a_switched_off_theme_package_draws_the_default_theme_quietly() {
    let (mut app, mut sent) = app();
    app.theme_pick_update(&chose("night"));
    assert!(asked(&mut sent).contains(&LIST_TAG));
    app.theme_pick_update(&listed(false));
    assert!(
        !asked(&mut sent).contains(&FILE_TAG),
        "关着的不去读主题文件"
    );
    assert_eq!(app.media.theme_pick.shown, None);
    assert!(app.typing.notice.is_none(), "不弹「读不成」");
    // 开回来：包开关的推送来了再查一次，去读主题文件。
    app.theme_pick_update(&Update::Packages(Vec::new()));
    app.theme_pick_update(&chose("night"));
    app.theme_pick_update(&listed(true));
    assert!(asked(&mut sent).contains(&FILE_TAG));
}
