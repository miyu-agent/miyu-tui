//! `$` 点名技能（发版前 check 26）：认 `$` 词、筛、从字里认出点了哪几个。

use super::{Picker, marks, named, token};
use crate::core::CoreSkill;

fn skills(names: &[&str]) -> Vec<CoreSkill> {
    names
        .iter()
        .map(|n| CoreSkill {
            name: (*n).into(),
            summary: format!("{n} 的说明"),
        })
        .collect()
}

#[test]
fn a_dollar_word_starts_the_line_or_follows_a_space() {
    let at = |text: &str| token(text, text.len());
    assert_eq!(at("$pd"), Some((0, "pd".into())));
    assert_eq!(at("看看 $re"), Some((7, "re".into())));
    assert_eq!(at("$"), Some((0, String::new())));
    assert_eq!(at("价格a$5"), None, "前面不是空白的不算");
    assert_eq!(at("$pdf 接着"), None, "到光标为止有空白就不是了");
    assert_eq!(token("$pdf 后面", 4), Some((0, "pdf".into())), "照光标算");
}

#[test]
fn named_skills_are_read_in_order_once_each() {
    let all = skills(&["pdf", "review", "review-pr"]);
    assert_eq!(
        named("$review-pr 先看， $pdf，再 $review-pr 一遍", &all),
        ["review-pr", "pdf"]
    );
    assert!(
        named("a$pdf $pdf2 $nope", &all).is_empty(),
        "要在空白后面、名字要整个对上"
    );
    // 上色照字节范围：`$` 起到名字完。
    assert_eq!(marks("用 $pdf。", &all), [(4, 8, "pdf".to_string())]);
}

#[test]
fn the_list_filters_by_name_and_stays_closed_after_esc_until_the_word_changes() {
    let all = skills(&["pdf", "review", "docx"]);
    let mut picker = Picker::default();
    let found = picker.find("$re", 3, &all).expect("开");
    assert_eq!(found.items[0].0.name, "review");
    assert_eq!(found.items[0].1, [0, 1], "对上的字");
    let everything = picker.find("$", 1, &all).expect("开");
    assert_eq!(everything.items.len(), 3, "空着的全列");
    picker.dismiss(&everything);
    assert!(
        picker.find("$", 1, &all).is_none(),
        "Esc 关了，词没变一直关着"
    );
    assert!(picker.find("$d", 2, &all).is_some(), "词变了再开");
    assert!(picker.find("$d", 2, &[]).is_none(), "没有技能可点的不开");
}
