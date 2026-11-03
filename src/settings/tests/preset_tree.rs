//! 预设窗里功能树的按键（「配置页」第 38 条「功能树」，核心设计 52）：`↑` `↓` 在一列里走、到顶回描述，`←` `→` 换列，
//! 组名那一行空格整组切。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::json;

use super::texts;
use crate::features::Row;
use crate::settings::Settings;
use crate::settings::pages::choose::PresetView;
use crate::settings::pages::preset_edit::{Field, fields};
use crate::settings::pages::presets::detail;
use crate::settings::popup::Popup;

/// 两组：基础系统两个功能，人格一个功能；排成两列。
fn view(cursor: usize) -> PresetView {
    let got = detail(&json!({"preset": "mine", "sections": [
        {"id": "basesystem", "kind": "group", "name": "基础系统", "items": [
            {"id": "read", "name": "读文件", "on": true},
            {"id": "write", "name": "新建文件", "on": false}]},
        {"id": "persona", "kind": "group", "name": "人格", "items": [
            {"id": "roleplay", "name": "人设遵循提醒", "on": true}]}]}));
    let view = PresetView {
        id: "mine".into(),
        detail: Some(got),
        error: None,
        cursor,
        two: Default::default(),
    };
    view.two.set(true);
    view
}

/// 按一个键，交回光标停在哪一格。
fn press(cursor: usize, code: KeyCode) -> Option<Field> {
    let mut page = Settings::default();
    page.preset_key(
        view(cursor),
        KeyEvent::new(code, KeyModifiers::NONE),
        &texts(),
    );
    let Some(Popup::Preset(after)) = page.popup else {
        panic!("窗关了");
    };
    let detail = after.detail.as_ref().unwrap();
    fields(detail, true).get(after.cursor).copied()
}

#[test]
fn keys_walk_a_column_and_jump_across() {
    // 能改的几格：名字 0、描述 1、左列 基础系统 2、读文件 3、新建文件 4，右列 人格 5、人设遵循提醒 6。
    let at = |i: usize| fields(view(0).detail.as_ref().unwrap(), true)[i];
    assert_eq!(at(2), Field::Row(Row::Section(0)));
    assert_eq!(
        press(1, KeyCode::Down),
        Some(Field::Row(Row::Section(0))),
        "描述往下进功能树"
    );
    assert_eq!(
        press(4, KeyCode::Down),
        Some(Field::Row(Row::Feature(1))),
        "左列到底不过到右列"
    );
    assert_eq!(
        press(5, KeyCode::Up),
        Some(Field::Summary),
        "右列顶上往上回描述"
    );
    assert_eq!(
        press(3, KeyCode::Right),
        Some(Field::Row(Row::Feature(2))),
        "换到右列最近的那一行"
    );
    assert_eq!(
        press(6, KeyCode::Char('h')),
        Some(Field::Row(Row::Feature(0)))
    );
}

#[test]
fn space_on_a_heading_saves_the_whole_section_at_once() {
    let mut page = Settings::default();
    page.preset_key(
        view(2),
        KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
        &texts(),
    );
    let asks = page.take_asks();
    assert_eq!(asks.len(), 1, "一次存");
    assert_eq!(
        asks[0].2["changes"],
        json!([{"key": "features.write", "value": true}]),
        "组里有关着的：全开"
    );
}
