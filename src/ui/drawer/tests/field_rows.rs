//! 写字的地方多高照抽屉能用的高度定、鼠标点一下放光标、长粘贴画成块（2026-11-27）。

use super::{Drawer, KeyCode, KeyEvent, KeyModifiers, lines, look, press, text, texts, view};
use ratatui::layout::{Position, Rect};
use unicode_width::UnicodeWidthStr;

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

fn ten_lines(d: &mut Drawer) {
    for i in 0..10 {
        if i > 0 {
            d.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
        }
        for c in format!("第{i}行").chars() {
            press(d, KeyCode::Char(c));
        }
    }
}

fn draw(d: &mut Drawer, area: Rect) -> crate::caret::Caret {
    let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 30)).unwrap();
    let mut caret = crate::caret::Caret::default();
    term.draw(|f| {
        super::super::draw(f, (area, 0), d, &texts(), &look(), false, &mut caret);
    })
    .unwrap();
    caret
}

#[test]
fn a_short_drawer_gives_the_field_fewer_rows() {
    let mut d = writing_other();
    ten_lines(&mut d);
    assert_eq!(
        d.fit_field(30),
        crate::drawer::FIELD_ROWS,
        "高的抽屉最多 6 行"
    );
    assert_eq!(d.fit_field(6), 3, "抽屉只能有 6 行：写字的地方最多一半");
    assert_eq!(d.fit_field(1), 1, "再矮也留一行");
    // 量抽屉多高（`rows`）的时候定下的，画的时候照它排。
    let tall = super::super::rows(&d, &texts(), &look(), 60, 8);
    assert_eq!(d.field_rows.get(), 4);
    let shown = lines(&d, 60);
    let at = shown.iter().position(|l| l == "› 2. 输入其他答案").unwrap();
    assert_eq!(
        shown[at + 1..at + 5],
        ["     第6行", "     第7行", "     第8行", "     第9行"]
    );
    assert!(shown[at + 5].is_empty(), "{shown:#?}");
    assert_eq!(tall, 8, "{shown:#?}");
}

#[test]
fn a_click_on_the_text_moves_the_cursor_there() {
    let mut d = writing_other();
    for c in "第一行".chars() {
        press(&mut d, KeyCode::Char(c));
    }
    d.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
    for c in "第二行".chars() {
        press(&mut d, KeyCode::Char(c));
    }
    let caret = draw(&mut d, Rect::new(2, 3, 50, 12));
    // 光标在第二行末尾：第一行在它上面一行，字从第 2 + 5 列起。
    let first = caret.at.y - 1;
    let field = d.field_mut().unwrap();
    assert!(!field.click(Position::new(0, first)), "点在字的左边不算");
    // 点在「一」的右半边：落在它后面。
    assert!(field.click(Position::new(2 + 5 + 3, first)));
    press(&mut d, KeyCode::Char('二'));
    assert_eq!(d.typed[0], "第一二行\n第二行");
    // 再画一遍，终端的光标跟着。
    let caret = draw(&mut d, Rect::new(2, 3, 50, 12));
    assert_eq!((caret.at.x, caret.at.y), (2 + 5 + 6, first));
}

#[test]
fn a_folded_paste_shows_as_a_chip() {
    let mut d = writing_other();
    let rule = crate::input::PasteRule {
        lines: 2,
        chars: 100,
        label: "[已粘贴 {lines} 行]".into(),
    };
    d.paste("一\n二\n三", &rule);
    let v = view(&d, &texts(), &look(), 60, false);
    let (row, col) = v.cursor.unwrap();
    assert_eq!(text(&v.lines[row]), "     [已粘贴 3 行]");
    assert_eq!(col, 5 + 13, "光标在块后面（块上的字占 13 列）");
    let chip = v.lines[row]
        .spans
        .iter()
        .find(|s| s.content.contains("已粘贴"))
        .unwrap();
    assert_eq!(chip.style, crate::theme::chip(), "块照输入框的样子画");
}

#[test]
fn writing_other_beside_a_preview_wraps_inside_the_options_column() {
    // 选项带文字画、并排的时候（`preview.rs`）：写的字折在左边那一列里，中间的竖线不被顶歪。
    let asked = serde_json::from_value(serde_json::json!({
        "call_id": "c", "questions": [{"question": "哪种排法？", "options": [
            {"label": "横排", "preview": "┌──────┐\n│ 甲 乙 │\n└──────┘"}, {"label": "竖排"}]}]
    }))
    .unwrap();
    let mut d = Drawer::question(None, asked);
    press(&mut d, KeyCode::Down);
    press(&mut d, KeyCode::Down);
    press(&mut d, KeyCode::Enter);
    for _ in 0..60 {
        press(&mut d, KeyCode::Char('x'));
    }
    let shown = lines(&d, 100);
    let written: Vec<&String> = shown.iter().filter(|l| l.contains("xx")).collect();
    assert!(written.len() >= 2, "折了行：{shown:#?}");
    let total: usize = written.iter().map(|l| l.matches('x').count()).sum();
    assert_eq!(total, 60, "{shown:#?}");
    let divider = shown[2].find('│').map(|i| shown[2][..i].width());
    assert!(divider.is_some(), "并排着：{shown:#?}");
    for l in &written {
        let at = l.find('│').map(|i| l[..i].width());
        assert_eq!(at, divider, "竖线对齐：{shown:#?}");
        assert!(l.rfind('x') < l.find('│'), "字在竖线左边：{shown:#?}");
    }
}
