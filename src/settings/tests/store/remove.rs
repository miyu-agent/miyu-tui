//! 卸载的弹窗是按钮（PK-5，2026-11-05 项目主人：「不应该是一个弹窗，然后给三个按钮吗？」）：默认停在「卸载」，`←`、`→`、
//! `Tab` 挪，`Enter` 按，`Esc` 取消；只有按「卸载并删除配置」才带 `purge`；没看完时卸的两个按钮不做、「取消」照样能按；
//! 核心没说配置目录的只有「卸载」「取消」。

use ratatui::crossterm::event::KeyCode;
use serde_json::{Value, json};

use super::{opened, press, read_through, sent, status};
use crate::settings::pages::store::remove::{Button, selected};
use crate::settings::pages::store::{AskView, Flow, Tab};
use crate::settings::{Settings, Tone};

/// 「已安装」上按 `d`、核心回了预览、预览块画过一帧：在问卸 `im`。
fn asking(config: bool) -> Settings {
    let mut page = opened();
    page.more.store.tab = Tab::Installed;
    press(&mut page, KeyCode::Char('d'));
    let preview = sent(&mut page);
    let mut got = json!({"package": "im", "version": "0.8.4"});
    if config {
        got["config"] = json!({"path": "/x/etc/im", "kept": true});
    }
    page.answer(preview[0].0, Ok(got), &super::texts());
    page.more.store.ask = read_through(3);
    page
}

fn removed(page: &mut Settings) -> Vec<Value> {
    sent(page)
        .into_iter()
        .filter(|(_, m, _)| *m == "package.remove")
        .map(|(_, _, p)| p)
        .collect()
}

#[test]
fn enter_on_the_default_button_removes_and_keeps_the_config() {
    let mut page = asking(true);
    assert_eq!(selected(&page.more.store), Some(Button::Keep));
    press(&mut page, KeyCode::Char('y'));
    assert!(sent(&mut page).is_empty(), "卸的弹窗不认 y");
    press(&mut page, KeyCode::Enter);
    assert_eq!(
        removed(&mut page),
        [json!({"package": "im"})],
        "不删配置的不写 purge：旧核心不认"
    );
}

#[test]
fn the_middle_button_removes_the_config_too() {
    let mut page = asking(true);
    press(&mut page, KeyCode::Right);
    assert_eq!(selected(&page.more.store), Some(Button::Purge));
    press(&mut page, KeyCode::Enter);
    assert_eq!(
        removed(&mut page),
        [json!({"package": "im", "purge": true})]
    );
}

#[test]
fn moving_stops_at_both_ends_and_cancel_does_nothing() {
    let mut page = asking(true);
    for _ in 0..5 {
        press(&mut page, KeyCode::Tab);
    }
    assert_eq!(selected(&page.more.store), Some(Button::Cancel));
    press(&mut page, KeyCode::BackTab);
    press(&mut page, KeyCode::Left);
    press(&mut page, KeyCode::Left);
    assert_eq!(selected(&page.more.store), Some(Button::Keep));
    press(&mut page, KeyCode::Right);
    press(&mut page, KeyCode::Right);
    press(&mut page, KeyCode::Enter);
    assert_eq!(page.more.store.flow, None);
    assert_eq!(status(&page), Some(("已取消".into(), Tone::Note)));
    assert!(removed(&mut page).is_empty());
}

#[test]
fn without_a_config_directory_there_are_two_buttons() {
    let mut page = asking(false);
    press(&mut page, KeyCode::Right);
    press(&mut page, KeyCode::Right);
    assert_eq!(selected(&page.more.store), Some(Button::Cancel));
    press(&mut page, KeyCode::Left);
    press(&mut page, KeyCode::Enter);
    assert_eq!(removed(&mut page), [json!({"package": "im"})]);
}

#[test]
fn unread_blocks_the_remove_buttons_but_not_cancel() {
    let mut page = asking(true);
    page.more.store.ask = AskView {
        top: 0,
        rows: 10,
        room: 4,
    };
    for right in [0, 1] {
        page.more.store.button = right;
        press(&mut page, KeyCode::Enter);
        assert!(removed(&mut page).is_empty(), "没看完不卸");
        assert_eq!(status(&page), Some(("还有 6 行未查看".into(), Tone::Note)));
    }
    press(&mut page, KeyCode::Char('j'));
    assert_eq!(page.more.store.ask.top, 1, "照旧能滚");
    press(&mut page, KeyCode::Right);
    press(&mut page, KeyCode::Enter);
    assert!(page.more.store.flow.is_none(), "取消随时能按");
    // 再进卸载：回到「卸载」。
    press(&mut page, KeyCode::Char('d'));
    assert_eq!(page.more.store.button, 0);
    assert!(matches!(
        page.more.store.flow,
        Some(Flow::Previewing { .. })
    ));
}
