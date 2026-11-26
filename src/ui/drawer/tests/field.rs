//! 写字的地方（2026-11-27 项目主人照 Claude Code）：「其他」在那一项下面一块，和标题对齐；软换行、换行另起一行（原来画成
//! `↵` 挤在一行、只留末尾一截）；长到上限以后在里面滚、光标那一行露着；空着写暗色的提示；写完以后折行显示，最多几行。

use super::{Drawer, KeyCode, KeyEvent, KeyModifiers, fit, lines, look, press, text, texts, view};
use unicode_width::UnicodeWidthStr;

fn shift_enter(d: &mut Drawer) {
    d.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
}

fn type_in(d: &mut Drawer, s: &str) {
    for c in s.chars() {
        press(d, KeyCode::Char(c));
    }
}

/// 一道单选题、一个选项，在「其他」上进了编辑。
fn writing_other() -> Drawer {
    let asked = serde_json::from_value(serde_json::json!({
        "call_id": "c", "questions": [{"question": "第一道？", "options": [{"label": "选项0"}]}]
    }))
    .unwrap();
    let mut d = Drawer::question(None, asked);
    press(&mut d, KeyCode::Down);
    press(&mut d, KeyCode::Enter);
    d
}

/// 「其他」那一行下面的几行（到空行为止）。
fn under_other(shown: &[String]) -> Vec<String> {
    let at = shown
        .iter()
        .position(|l| l == "› 2. 输入其他答案")
        .unwrap_or_else(|| panic!("「其他」的标题照旧：{shown:#?}"));
    shown[at + 1..]
        .iter()
        .take_while(|l| !l.is_empty())
        .cloned()
        .collect()
}

#[test]
fn other_wraps_softly_under_its_title() {
    let mut d = writing_other();
    type_in(&mut d, &"a".repeat(40));
    // 宽 30：标题前面 `› 2. ` 五列，留一列给光标，一行 24 个。
    let v = view(&d, &texts(), &look(), 30, false);
    let shown: Vec<String> = v.lines.iter().map(text).collect();
    let rows = under_other(&shown);
    assert_eq!(
        rows,
        [
            format!("     {}", "a".repeat(24)),
            format!("     {}", "a".repeat(16))
        ],
        "折到下一行，和标题对齐，不只留末尾一截：{shown:#?}"
    );
    let (row, col) = v.cursor.unwrap();
    assert_eq!((text(&v.lines[row]), col), (rows[1].clone(), 5 + 16));
    // 中文一个字两列：12 个一行，第 13 个折下去。
    let mut d = writing_other();
    type_in(&mut d, &"中".repeat(13));
    let shown = lines(&d, 30);
    let rows = under_other(&shown);
    assert_eq!(
        rows,
        [format!("     {}", "中".repeat(12)), "     中".to_string()]
    );
    assert!(rows.iter().all(|r| r.width() <= 29), "{rows:#?}");
}

#[test]
fn a_new_line_starts_a_new_row_instead_of_a_return_mark() {
    let mut d = writing_other();
    type_in(&mut d, "甲");
    shift_enter(&mut d);
    type_in(&mut d, "乙");
    let shown = lines(&d, 60);
    assert_eq!(under_other(&shown), ["     甲", "     乙"]);
    assert!(!shown.iter().any(|l| l.contains('↵')), "{shown:#?}");
    // 光标挪回第一行：终端的光标跟着。
    press(&mut d, KeyCode::Up);
    let v = view(&d, &texts(), &look(), 60, false);
    let (row, col) = v.cursor.unwrap();
    assert_eq!((text(&v.lines[row]).as_str(), col), ("     甲", 7));
}

#[test]
fn the_field_grows_to_six_rows_then_scrolls_with_the_cursor() {
    let mut d = writing_other();
    for i in 0..10 {
        if i > 0 {
            shift_enter(&mut d);
        }
        type_in(&mut d, &format!("第{i}行"));
    }
    let rows = under_other(&lines(&d, 60));
    let want: Vec<String> = (4..10).map(|i| format!("     第{i}行")).collect();
    assert_eq!(rows, want, "最多 6 行，露着光标所在的最后一行");
    for _ in 0..9 {
        press(&mut d, KeyCode::Up);
    }
    let v = view(&d, &texts(), &look(), 60, false);
    let shown: Vec<String> = v.lines.iter().map(text).collect();
    let want: Vec<String> = (0..6).map(|i| format!("     第{i}行")).collect();
    assert_eq!(under_other(&shown), want, "往上走滚回去");
    let (row, _) = v.cursor.unwrap();
    assert_eq!(shown[row], "     第0行");
}

#[test]
fn empty_fields_show_a_dim_placeholder() {
    let d = writing_other();
    let v = view(&d, &texts(), &look(), 60, false);
    let (row, col) = v.cursor.unwrap();
    assert_eq!(text(&v.lines[row]), "     输入其他答案");
    assert_eq!(col, 5, "光标停在提示的第一个字上");
    let hint = v.lines[row].spans.last().unwrap();
    assert_eq!(hint.style, crate::theme::dim());
    // 理由：「不允许」后面空两格写暗色的「理由（可选）」。
    let approval = serde_json::from_value(serde_json::json!({
        "call_id": "c1", "access": "execute", "detail": {"tool": "shell", "command": "rm -rf build"}
    }))
    .unwrap();
    let mut d = Drawer::approval(None, approval);
    press(&mut d, KeyCode::Char('n'));
    let v = view(&d, &texts(), &look(), 60, false);
    let (row, col) = v.cursor.unwrap();
    assert_eq!(text(&v.lines[row]), "› 2. 不允许  理由（可选）");
    assert_eq!(col, 13);
    assert_eq!(
        v.lines[row].spans.last().unwrap().style,
        crate::theme::dim()
    );
}

#[test]
fn written_text_shows_its_lines_when_not_editing_and_stops_at_three() {
    let mut d = writing_other();
    type_in(&mut d, "第一行");
    shift_enter(&mut d);
    type_in(&mut d, "第二行");
    press(&mut d, KeyCode::Esc);
    let shown = lines(&d, 60);
    let at = shown.iter().position(|l| l == "› 2. 第一行").unwrap();
    assert_eq!(shown[at + 1], "     第二行", "{shown:#?}");
    assert!(!shown.iter().any(|l| l.contains('↵')), "{shown:#?}");
    // 五行的：只露三行，最后一行接「…」。
    let mut d = writing_other();
    for i in 0..5 {
        if i > 0 {
            shift_enter(&mut d);
        }
        type_in(&mut d, &format!("行{i}"));
    }
    press(&mut d, KeyCode::Esc);
    let shown = lines(&d, 60);
    let at = shown.iter().position(|l| l == "› 2. 行0").unwrap();
    assert_eq!(shown[at + 1], "     行1");
    assert_eq!(shown[at + 2], "     行2…");
    assert!(shown[at + 3].is_empty(), "{shown:#?}");
}

#[test]
fn notes_wrap_and_line_up_under_their_first_row() {
    let mut d = writing_other();
    press(&mut d, KeyCode::Esc);
    press(&mut d, KeyCode::Char('n'));
    type_in(&mut d, &"b".repeat(30));
    // 宽 30：`  补充  ` 八列，留一列给光标，一行 21 个。
    let v = view(&d, &texts(), &look(), 30, false);
    let shown: Vec<String> = v.lines.iter().map(text).collect();
    let at = shown
        .iter()
        .position(|l| l.starts_with("  补充  "))
        .unwrap();
    assert_eq!(shown[at], format!("  补充  {}", "b".repeat(21)));
    assert_eq!(shown[at + 1], format!("        {}", "b".repeat(9)));
    assert_eq!(v.cursor, Some((at + 1, 8 + 9)));
}

#[test]
fn a_tight_drawer_keeps_the_row_with_the_cursor() {
    let mut d = writing_other();
    press(&mut d, KeyCode::Esc);
    press(&mut d, KeyCode::Char('n'));
    for i in 0..4 {
        if i > 0 {
            shift_enter(&mut d);
        }
        type_in(&mut d, &format!("补{i}"));
    }
    let mut scroll = 0;
    let v = fit(view(&d, &texts(), &look(), 60, false), 7, &mut scroll);
    let (row, _) = v.cursor.expect("光标那一行露着");
    assert!(text(&v.lines[row]).ends_with("补3"), "{:#?}", v.lines);
}

#[test]
fn the_terminal_cursor_sits_where_the_editor_cursor_is() {
    let mut d = writing_other();
    type_in(&mut d, "abc");
    press(&mut d, KeyCode::Left);
    let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 12)).unwrap();
    let mut caret = crate::caret::Caret::default();
    term.draw(|f| {
        let area = ratatui::layout::Rect::new(3, 1, 50, 10);
        super::super::draw(f, (area, 0), &mut d, &texts(), &look(), false, &mut caret);
    })
    .unwrap();
    let buffer = term.backend().buffer().clone();
    assert!(caret.shown);
    let row: String = (3..20)
        .map(|x| buffer[(x, caret.at.y)].symbol().to_string())
        .collect();
    assert_eq!(row.trim_end(), "     abc", "光标所在的那一行是写字的那一行");
    assert_eq!(caret.at.x, 3 + 5 + 2, "停在 b 和 c 中间");
}
