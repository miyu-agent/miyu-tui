//! `/usage` 换页的键（蓝图「配置与模型」第 5 条）：`Tab`、`Shift+Tab`、`←` `→` 换页；不能打字的框里 `h`、`l` 同 `←` `→`。
//! 照真按键走（`handle`），按键模式是 Vim 的也一样。

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::app::Panel;

fn press(app: &mut crate::app::App, code: KeyCode) {
    app.handle(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
}

fn tab(app: &crate::app::App) -> usize {
    match app.boxes.panel {
        Some(Panel::Usage { tab, .. }) => tab,
        other => panic!("用量框开着：{other:?}"),
    }
}

#[test]
fn arrows_and_h_l_turn_the_usage_pages_in_either_key_mode() {
    for vim in [false, true] {
        let (mut app, _sent) = crate::app::test_support::app();
        app.config.layout.vim = vim;
        app.talk.transcript.session = Some("s1".into());
        app.open_usage();
        press(&mut app, KeyCode::Right);
        assert_eq!(tab(&app), 1, "→ 下一页（vim {vim}）");
        press(&mut app, KeyCode::Left);
        assert_eq!(tab(&app), 0, "← 上一页（vim {vim}）");
        press(&mut app, KeyCode::Char('l'));
        assert_eq!(tab(&app), 1, "l 下一页（vim {vim}）");
        press(&mut app, KeyCode::Char('h'));
        assert_eq!(tab(&app), 0, "h 上一页（vim {vim}）");
        press(&mut app, KeyCode::Tab);
        assert_eq!(tab(&app), 1, "Tab 照旧（vim {vim}）");
    }
}
