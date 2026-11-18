//! 示范对话的两格窗和列表：两项的名字宽度不一样时（「你问的」「AI 回的」），后面的内容照旧从同一列起。

use unicode_width::UnicodeWidthStr;

use crate::config::Config;
use crate::settings::pages::choose::PersonaView;
use crate::settings::pages::dialogs::{DialogsView, PairEdit};
use crate::ui::settings::dialog_popups::{dialogs, pair};
use crate::ui::settings::popup::Lines;

/// 一行里的字拼起来。
fn text(line: &ratatui::text::Line<'_>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

/// 内容从第几列起：名字、两个空格之后。
fn value_col(row: &str, side: &str) -> usize {
    let at = row.find(side).expect("行里有名字") + side.len();
    let rest = &row[at..];
    row[..at].width() + (rest.len() - rest.trim_start().len())
}

fn back() -> PersonaView {
    PersonaView {
        id: "p".into(),
        detail: None,
        error: None,
        cursor: 0,
    }
}

#[test]
fn the_two_sides_of_a_pair_start_their_values_in_the_same_column() {
    let texts = Config::builtin().unwrap().text.settings;
    let sides = &texts.more.dialogs.sides;
    let view = DialogsView {
        persona: "p".into(),
        pairs: Vec::new(),
        version: None,
        sel: 0,
        back: back(),
    };
    let edit = PairEdit {
        sides: crate::pairs::opened(&crate::pairs::Pair {
            user: "甲".into(),
            assistant: "乙".into(),
        }),
        focus: 0,
        index: None,
        error: None,
        back: view,
    };
    let mut lines = Lines::default();
    pair(&mut lines, &edit, &texts);
    let rows: Vec<String> = lines.body.iter().map(|b| text(&b.line)).collect();
    assert_eq!(
        value_col(&rows[0], &sides[0]),
        value_col(&rows[1], &sides[1]),
        "两格窗里两项的内容要对齐：{rows:?}"
    );
}

#[test]
fn the_two_sides_of_the_list_start_their_values_in_the_same_column() {
    let texts = Config::builtin().unwrap().text.settings;
    let sides = &texts.more.dialogs.sides;
    let view = DialogsView {
        persona: "p".into(),
        pairs: vec![crate::pairs::Pair {
            user: "甲".into(),
            assistant: "乙".into(),
        }],
        version: None,
        sel: 0,
        back: back(),
    };
    let mut lines = Lines::default();
    dialogs(&mut lines, &view, &texts, 80);
    let rows: Vec<String> = lines.body.iter().map(|b| text(&b.line)).collect();
    assert_eq!(
        value_col(&rows[0], &sides[0]),
        value_col(&rows[1], &sides[1]),
        "列表里两项的内容要对齐：{rows:?}"
    );
}
