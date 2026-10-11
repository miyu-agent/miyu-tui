use ratatui::style::Color;

use super::read;
use crate::mascot::Part;

#[test]
fn an_empty_model_is_the_built_in_mascot() {
    let look = read(b"{}").unwrap();
    let builtin = crate::config::Config::builtin().unwrap().mascot;
    assert_eq!((look.cols, look.rows), (builtin.cols, builtin.rows));
    assert_eq!(look.shapes.len(), builtin.shapes.len());
    assert_eq!(look.colors.of(Part::Head), None, "内置的照主题的颜色");
}

#[test]
fn what_a_package_writes_covers_the_built_in_one_field_at_a_time() {
    let look = read(
        br##"{"ramp": " xX", "idle": {"blink_ms": [50, 60]},
             "colors": {"head": "#ffcc00", "fin": "lightred"}}"##,
    )
    .unwrap();
    assert_eq!(look.ramp, " xX");
    assert_eq!(look.idle.blink_ms, [50, 60]);
    let builtin = crate::config::Config::builtin().unwrap().mascot;
    assert_eq!(
        look.idle.blink_every_ms, builtin.idle.blink_every_ms,
        "idle 里没写的照内置的"
    );
    assert_eq!(
        look.colors.of(Part::Head),
        Some(Color::Rgb(0xff, 0xcc, 0x00))
    );
    assert_eq!(look.colors.of(Part::Ear), None);
}

#[test]
fn a_model_that_breaks_a_rule_is_refused_whole() {
    let refused = |text: &str| read(text.as_bytes()).unwrap_err();
    assert!(refused(r#"{"perch": {"settle_ms": 1, "row_ms": 1}}"#).contains("perch"));
    assert!(refused(r#"{"cell_aspect": 1.0}"#).contains("cell_aspect"));
    assert!(refused(r#"{"tail": 1}"#).contains("tail"));
    assert!(refused(r#"{"cols": 40}"#).contains("cols"));
    assert!(refused(r#"{"shapes": []}"#).contains("shapes"));
    assert!(refused(r#"{"idle": {"wag_ms": 3}}"#).contains("wag_ms"));
    assert!(!refused(r##"{"colors": {"head": "#zzzzzz"}}"##).is_empty());
    assert!(refused("[1]").contains("object"));
    assert!(refused("not json").contains("JSON"));
    let big = format!(r#"{{"ramp": "{}"}}"#, "x".repeat(super::MAX_BYTES));
    assert!(refused(&big).contains("larger"));
}

#[test]
fn control_or_wide_characters_cannot_reach_the_terminal() {
    // review A3（2026-10-11）：明暗那串字、眼睛和嘴的字经 set_char 直接进画面缓冲，绕过了画面库的控制字符过滤；
    // 一个纯数据的吉祥物包写进 ESC 就能重置、操控终端。控制字符、不占一格的字整份拒掉。
    let esc = char::from(0x1b);
    assert!(read(format!(r#"{{"ramp": " a{esc}c"}}"#).as_bytes()).is_err());
    assert!(read(format!(r#"{{"face": {{"eye_mark": "{esc}"}}}}"#).as_bytes()).is_err());
    assert!(
        read(r#"{"face": {"line_mark": "中"}}"#.as_bytes()).is_err(),
        "占两格的"
    );
    assert!(read(r#"{"ramp": " .oO"}"#.as_bytes()).is_ok());
}
