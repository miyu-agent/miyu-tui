//! 悬浮窗里的单行输入框比窗宽时（蓝图「配置页」第 15 条，审计 #084）：只露光标附近那一截，碰到左右边才挪，同主输入框碰到
//! 上下边才滚；原来整串从头画、窗右边截掉，光标夹在右边，后半截打的字看不见。

use ratatui::crossterm::event::KeyCode;

use super::{page, press, screen};
use crate::config::Config;
use crate::settings::Settings;

/// 一百多个字、最后是 `XYZ`：比窗宽，认得出是哪一截。
fn long() -> String {
    format!("{}XYZ", "abcdefghij".repeat(10))
}

/// 光标那一行写的字、光标在第几列。
fn caret_row(p: &mut Settings, config: &Config) -> (String, usize) {
    let (text, caret) = screen(p, config, 120, 34);
    assert!(caret.shown, "正在改：光标露出来");
    let row = text
        .lines()
        .nth(usize::from(caret.at.y))
        .unwrap_or_default()
        .to_string();
    (row, usize::from(caret.at.x))
}

/// 从第 `col` 列起写的字（宽字占两列）。
fn from_col(row: &str, col: usize) -> String {
    let mut at = 0;
    row.chars()
        .skip_while(|c| {
            let before = at;
            at += unicode_width::UnicodeWidthChar::width(*c).unwrap_or(0);
            before < col
        })
        .collect()
}

/// `piece` 结尾在第几列。
fn end_col(row: &str, piece: &str) -> Option<usize> {
    row.find(piece)
        .map(|at| unicode_width::UnicodeWidthStr::width(&row[..at + piece.len()]))
}

/// 开供应商的窗、在显示名称上开始改（同 `typing_in_a_form_field_is_not_underlined`），打 `typed`。
fn typing_a_name(config: &Config, typed: &str) -> Settings {
    let mut p = page(config);
    press(&mut p, config, KeyCode::Enter);
    press(&mut p, config, KeyCode::Down);
    press(&mut p, config, KeyCode::Enter);
    press(&mut p, config, KeyCode::Enter);
    for c in typed.chars() {
        press(&mut p, config, KeyCode::Char(c));
    }
    p
}

#[test]
fn a_long_value_in_a_form_shows_the_part_around_the_caret() {
    let config = Config::builtin().unwrap();
    let mut p = typing_a_name(&config, &long());
    let (row, x) = caret_row(&mut p, &config);
    assert_eq!(
        end_col(&row, "XYZ"),
        Some(x),
        "光标紧跟着刚打的字：{row:?} 光标 {x}"
    );
    assert!(row.contains("显示名称"), "名字那一截不跟着滚：{row:?}");
    // 回到开头：露出头一截。
    press(&mut p, &config, KeyCode::Home);
    let (row, x) = caret_row(&mut p, &config);
    assert!(from_col(&row, x).contains("abcdefghij"), "{row:?} 光标 {x}");
    assert!(!row.contains("XYZ"), "{row:?}");
}

#[test]
fn moving_back_inside_the_field_does_not_scroll_until_the_left_edge() {
    let config = Config::builtin().unwrap();
    let mut p = typing_a_name(&config, &long());
    let (at_end, x) = caret_row(&mut p, &config);
    for _ in 0..3 {
        press(&mut p, &config, KeyCode::Left);
    }
    let (row, moved) = caret_row(&mut p, &config);
    assert_eq!(row, at_end, "往回在框里不挪");
    assert_eq!(moved + 3, x, "光标往左三格");
    // 一路往左：碰到框的左边以后跟着滚，到头光标停在框的头一格，露出开头。
    press(&mut p, &config, KeyCode::Home);
    let (_, home) = caret_row(&mut p, &config);
    press(&mut p, &config, KeyCode::End);
    for _ in 0..200 {
        press(&mut p, &config, KeyCode::Left);
    }
    let (row, x) = caret_row(&mut p, &config);
    assert_eq!(x, home, "{row:?}");
    assert!(
        from_col(&row, x).contains("abcdefghij") && !row.contains("XYZ"),
        "{row:?}"
    );
}

#[test]
fn the_one_line_edit_window_scrolls_the_same_way() {
    use crate::settings::pages::choose::LineEdit;
    use crate::settings::popup::Popup;
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    let mut edit = LineEdit::open("im.base_url", "地址", false, &serde_json::Value::Null);
    edit.editor.set(&long());
    p.popup = Some(Popup::Line(edit));
    let (row, x) = caret_row(&mut p, &config);
    assert_eq!(end_col(&row, "XYZ"), Some(x), "{row:?} 光标 {x}");
}
